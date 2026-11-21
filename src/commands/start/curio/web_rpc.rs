//! Local Curio GUI transport and per-SP administrator sessions.

use crate::commands::start::step::SetupContext;
use crate::paths::foc_devnet_curio_sp_volume;
use crate::utils::retry::{retry_with_fixed_delay, DEFAULT_MAX_RETRIES, DEFAULT_RETRY_DELAY_SECS};
use reqwest::blocking::{Client, Response};
use reqwest::header::{COOKIE, SET_COOKIE};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub(super) struct WebRpc {
    client: Client,
    base: String,
    session: Option<String>,
}

#[derive(Deserialize)]
struct AuthStatus {
    configured: bool,
    #[serde(rename = "authenticated")]
    _authenticated: bool,
}

#[derive(Serialize, Deserialize)]
struct Credentials {
    username: String,
    password: String,
}

impl WebRpc {
    pub(super) fn connect(context: &SetupContext, sp_index: usize) -> Result<Self> {
        let port: u16 = context
            .get(&format!("pdp_sp_{}_gui_port", sp_index))
            .ok_or("Curio GUI port missing")?
            .parse()?;
        let path = foc_devnet_curio_sp_volume(context.run_id(), sp_index).join("gui-admin.json");
        Self::connect_local(port, &path, sp_index)
    }

    fn connect_local(port: u16, path: &Path, sp_index: usize) -> Result<Self> {
        // Only this loopback GUI client accepts Curio's self-signed certificate.
        let client = Client::builder()
            .danger_accept_invalid_certs(true)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(2))
            .timeout(Duration::from_secs(30))
            .build()?;
        let mut rpc = Self {
            client,
            base: format!("https://127.0.0.1:{}", port),
            session: None,
        };
        let status = retry_with_fixed_delay(
            || rpc.probe(port),
            DEFAULT_MAX_RETRIES,
            DEFAULT_RETRY_DELAY_SECS,
            &format!("Curio SP {} GUI readiness", sp_index),
        )?;
        if let Some(status) = status {
            let credentials = credentials(path, status.configured)?;
            let action = if status.configured { "login" } else { "setup" };
            let response = rpc
                .client
                .post(format!("{}/api/auth/{}", rpc.base, action))
                .json(&json!({
                    "username": credentials.username,
                    "password": credentials.password,
                    "confirm": credentials.password,
                }))
                .send()?;
            if !response.status().is_success() {
                return Err(format!(
                    "Curio SP {} {} failed: HTTP {}",
                    sp_index,
                    action,
                    response.status()
                )
                .into());
            }
            rpc.session = Some(session_cookie(&response)?);
        }
        Ok(rpc)
    }

    fn probe(&mut self, port: u16) -> Result<Option<AuthStatus>> {
        let https = self
            .client
            .get(format!("https://127.0.0.1:{}/api/auth/status", port))
            .send();
        match https {
            Ok(response) => {
                let status = response.error_for_status()?.json::<AuthStatus>()?;
                self.base = format!("https://127.0.0.1:{}", port);
                Ok(Some(status))
            }
            Err(https_error) if https_error.is_connect() => {
                // Older Curio cannot establish TLS. Confirm it answers a read-only
                // Web RPC before sending a key over HTTP.
                let base = format!("http://127.0.0.1:{}", port);
                self.check_http_version(&base).map_err(|http_error| {
                    format!("Curio GUI HTTPS unavailable ({https_error}); legacy HTTP probe failed ({http_error})")
                })?;
                self.base = base;
                Ok(None)
            }
            Err(error) => Err(error.into()),
        }
    }

    fn check_http_version(&self, base: &str) -> Result<()> {
        let value: Value = self
            .client
            .post(format!("{}/api/webrpc/v0", base))
            .json(&json!({"jsonrpc":"2.0", "method":"CurioWeb.Version", "params":[], "id":1}))
            .send()?
            .error_for_status()?
            .json()?;
        if value["jsonrpc"] == "2.0"
            && value["id"] == 1
            && value["error"].is_null()
            && value["result"].as_str().is_some_and(|v| !v.is_empty())
        {
            Ok(())
        } else {
            Err("Curio legacy GUI did not return a JSON-RPC version result".into())
        }
    }

    pub(super) fn import_pdp_key(&self, private_key: &str) -> Result<String> {
        let mut request = self.client.post(format!("{}/api/webrpc/v0", self.base))
            .json(&json!({"jsonrpc":"2.0", "method":"CurioWeb.ImportPDPKey", "params":[private_key], "id":1}));
        if let Some(session) = &self.session {
            request = request.header(COOKIE, session);
        }
        let response = request.send()?;
        if !response.status().is_success() {
            return Err(format!("Curio ImportPDPKey failed: HTTP {}", response.status()).into());
        }
        let value: Value = response
            .json()
            .map_err(|_| "Curio ImportPDPKey returned invalid JSON")?;
        if value.get("error").is_some_and(|e| !e.is_null()) {
            // Do not echo server messages which might contain the private key.
            return Err(match value["error"]["code"].as_i64() {
                Some(code) => format!("Curio ImportPDPKey RPC failed (code {code})"),
                None => "Curio ImportPDPKey RPC failed".to_owned(),
            }
            .into());
        }
        value["result"]
            .as_str()
            .filter(|address| address.parse::<ethers_core::types::Address>().is_ok())
            .map(str::to_owned)
            .ok_or_else(|| "Curio ImportPDPKey response missing address".into())
    }
}

fn session_cookie(response: &Response) -> Result<String> {
    response
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .filter_map(|h| h.to_str().ok())
        .filter_map(|h| h.split(';').next())
        .find(|h| h.starts_with("curio_session=") && h.len() > "curio_session=".len())
        .map(str::to_owned)
        .ok_or_else(|| "Curio authentication response missing session cookie".into())
}

fn credentials(path: &Path, configured: bool) -> Result<Credentials> {
    match fs::read(path) {
        Ok(bytes) => return Ok(serde_json::from_slice(&bytes)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    if configured {
        return Err(format!(
            "Curio administrator already configured but credentials are missing: {}",
            path.display()
        )
        .into());
    }
    let value = Credentials {
        username: "devnet-admin".into(),
        password: hex::encode(rand::random::<[u8; 32]>()),
    };
    // Publish complete credentials atomically before setup. A failed write must
    // not leave a truncated file that breaks the next startup.
    let parent = path
        .parent()
        .ok_or("Curio credentials path has no parent")?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(&serde_json::to_vec(&value)?)?;
    file.as_file().sync_all()?;
    file.persist_noclobber(path)?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::net::TcpListener;
    use std::os::unix::fs::PermissionsExt;
    use std::thread;

    /// Run against a started devnet with FOC_DEVNET_BASEDIR and
    /// FOC_CURIO_TEST_RUN_DIR pointing to that devnet's run directory.
    #[test]
    #[ignore = "requires a running Curio devnet"]
    fn live_admin_login_for_both_providers() {
        let run_dir = std::path::PathBuf::from(std::env::var("FOC_CURIO_TEST_RUN_DIR").unwrap());
        let state: std::collections::HashMap<String, String> =
            serde_json::from_slice(&fs::read(run_dir.join("step_context.json")).unwrap()).unwrap();
        let run_id = run_dir.file_name().unwrap().to_str().unwrap();
        for sp in 1..=2 {
            let key = format!("pdp_sp_{}_gui_port", sp);
            let port: u16 = state.get(&key).unwrap().parse().unwrap();
            let path = foc_devnet_curio_sp_volume(run_id, sp).join("gui-admin.json");
            let rpc = WebRpc::connect_local(port, &path, sp).unwrap();
            let session = rpc.session.as_ref().expect("new Curio requires a session");
            let value: Value = rpc
                .client
                .post(format!("{}/api/webrpc/v0", rpc.base))
                .header(COOKIE, session)
                .json(&json!({"jsonrpc":"2.0","method":"CurioWeb.Version","params":[],"id":1}))
                .send()
                .unwrap()
                .error_for_status()
                .unwrap()
                .json()
                .unwrap();
            assert!(value["result"].as_str().is_some_and(|v| !v.is_empty()));
            let unauthenticated = rpc
                .client
                .post(format!("{}/api/webrpc/v0", rpc.base))
                .json(&json!({"jsonrpc":"2.0","method":"CurioWeb.Version","params":[],"id":1}))
                .send()
                .unwrap();
            assert_eq!(unauthenticated.status(), reqwest::StatusCode::UNAUTHORIZED);
        }
    }

    #[test]
    fn legacy_http_requires_a_valid_version_response() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            // An old HTTP server cannot complete the initial TLS handshake.
            let (mut tls, _) = listener.accept().unwrap();
            let mut buffer = [0; 4096];
            assert!(tls.read(&mut buffer).unwrap() > 0);
            drop(tls);
            let (mut http, _) = listener.accept().unwrap();
            assert!(http.read(&mut buffer).unwrap() > 0);
            let body = r#"{"jsonrpc":"2.0","id":1,"result":"1.29.2"}"#;
            write!(
                http,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });
        let mut rpc = WebRpc {
            client: Client::builder()
                .no_proxy()
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap(),
            base: String::new(),
            session: None,
        };
        assert!(rpc.probe(port).unwrap().is_none());
        assert_eq!(rpc.base, format!("http://127.0.0.1:{}", port));
        server.join().unwrap();
    }

    #[test]
    fn credentials_survive_restart_and_are_private() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("gui-admin.json");
        let initial = credentials(&path, false).unwrap();
        let restored = credentials(&path, true).unwrap();
        assert_eq!(initial.password, restored.password);
        assert!(initial.password.len() >= 12);
        assert_eq!(
            fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[test]
    fn existing_admin_without_credentials_is_not_overwritten() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("gui-admin.json");
        assert!(credentials(&path, true).is_err());
        assert!(!path.exists());
    }

    #[test]
    fn import_sends_session_and_does_not_echo_rpc_error_secrets() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = Vec::new();
            let mut buffer = [0; 4096];
            loop {
                let count = stream.read(&mut buffer).unwrap();
                assert!(count > 0);
                request.extend_from_slice(&buffer[..count]);
                if let Some(end) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&request[..end]).to_lowercase();
                    let length: usize = headers
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length: "))
                        .unwrap()
                        .parse()
                        .unwrap();
                    if request.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            let request = String::from_utf8(request).unwrap();
            assert!(request
                .to_lowercase()
                .contains("cookie: curio_session=test-session"));
            assert!(request.contains("CurioWeb.ImportPDPKey"));
            let body =
                r#"{"jsonrpc":"2.0","id":1,"error":{"code":-1,"message":"secret-private-key"}}"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });
        let rpc = WebRpc {
            client: Client::builder()
                .no_proxy()
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap(),
            base: format!("http://{}", address),
            session: Some("curio_session=test-session".into()),
        };
        let error = rpc
            .import_pdp_key("secret-private-key")
            .unwrap_err()
            .to_string();
        assert!(error.contains("-1"));
        assert!(!error.contains("secret-private-key"));
        server.join().unwrap();
    }
}
