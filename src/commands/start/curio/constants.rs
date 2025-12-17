//! Constants for Curio step configuration.

/// Curio Web RPC API port
pub const CURIO_WEB_RPC_PORT: u16 = 4701;

/// Curio PDP subsystem port
pub const CURIO_PDP_PORT: u16 = 4702;

/// Curio CLI machine address
pub const CURIO_CLI_MACHINE_ADDR: &str = "127.0.0.1:12300";

/// Curio storage paths inside container
pub const CURIO_FAST_STORAGE_PATH: &str = "/home/foc-user/curio/fast-storage";
pub const CURIO_LONG_TERM_STORAGE_PATH: &str = "/home/foc-user/curio/long-term-storage";

/// Curio layers configuration
pub const CURIO_LAYERS: &str = "seal,post,pdp-only,gui";

/// PDP layer configuration template
pub const PDP_LAYER_CONFIG_TEMPLATE: &str = r#"[HTTP]
DelegateTLS = true
DomainName = "pdp-sp-{sp_index}.foc-localnet.internal"
Enable = true
ListenAddress = "0.0.0.0:4702"

[Subsystems]
EnableCommP = true
EnableMoveStorage = true
EnablePDP = true
EnableParkPiece = true
"#;

/// Wait times (in seconds)
pub const DB_SETUP_WAIT_SECS: u64 = 10;
pub const DAEMON_STARTUP_WAIT_SECS: u64 = 15;
pub const STORAGE_ATTACH_WAIT_SECS: u64 = 5;
pub const PDP_KEY_IMPORT_WAIT_SECS: u64 = 5;

/// Verification test file size
pub const TEST_FILE_SIZE_BYTES: usize = 1024; // 1KB
