//! Resolves `LatestTag` to a concrete `GitTag` at init time by fetching
//! the newest tag on a specified branch from the remote repository.

use crate::config::Location;
use std::process::Command;
use tracing::info;

/// Throwaway bare repo in a temp directory, cleaned up on drop.
struct TempBareRepo(std::path::PathBuf);

impl TempBareRepo {
    fn create() -> Result<Self, Box<dyn std::error::Error>> {
        let dir = std::env::temp_dir().join(format!(
            "foc-devnet-tag-probe-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let status = Command::new("git")
            .args(["init", "--bare", dir.to_str().unwrap()])
            .env("GIT_TERMINAL_PROMPT", "0")
            .status()?;
        if !status.success() {
            return Err("git init --bare failed".into());
        }
        Ok(Self(dir))
    }

    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for TempBareRepo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Resolve `LatestTag` locations against the remote; pass others through unchanged.
pub fn resolve_location(location: Location) -> Result<Location, Box<dyn std::error::Error>> {
    match location {
        Location::LatestTag { url, branch } => {
            let tag = fetch_latest_tag(&url, &branch)?;
            info!(
                "Resolved latesttag for {} (branch {}) → {}",
                url, branch, tag
            );
            Ok(Location::GitTag { url, tag })
        }
        other => Ok(other),
    }
}

/// Blobless-fetch branch + tags into a temp bare repo, return the newest tag.
fn fetch_latest_tag(url: &str, branch: &str) -> Result<String, Box<dyn std::error::Error>> {
    info!("Fetching newest tag on branch '{}' from {}", branch, url);

    let repo = TempBareRepo::create()?;
    let refspec = format!("refs/heads/{b}:refs/heads/{b}", b = branch);

    let fetch = Command::new("git")
        .args(["fetch", "--tags", "--filter=blob:none", url, &refspec])
        .current_dir(repo.path())
        .env("GIT_TERMINAL_PROMPT", "0")
        .status()?;
    if !fetch.success() {
        return Err(format!("git fetch failed for {} (branch {})", url, branch).into());
    }

    let tags = Command::new("git")
        .args(["tag", "--merged", branch, "--sort=-creatordate"])
        .current_dir(repo.path())
        .output()?;
    if !tags.status.success() {
        return Err(format!(
            "git tag --merged {} failed: {}",
            branch,
            String::from_utf8_lossy(&tags.stderr).trim()
        )
        .into());
    }

    parse_newest_tag(&String::from_utf8_lossy(&tags.stdout), url, branch)
}

/// First non-empty line from `--sort=-creatordate` tag output.
fn parse_newest_tag(
    stdout: &str,
    url: &str,
    branch: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    stdout
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("No tags found on branch '{}' for {}", branch, url).into())
}
