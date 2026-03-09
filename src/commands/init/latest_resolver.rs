//! Resolves `LatestTag` → `GitTag` at init time.
//!
//! When a user specifies `--lotus latesttag:master` (or similar), we need to
//! figure out which concrete tag that maps to. This module:
//!
//! 1. Creates a temporary bare git repo (no working tree, no blobs).
//! 2. Fetches only the requested branch + tags from the remote.
//! 3. Picks the newest tag reachable from that branch.
//! 4. Returns a `GitTag` so the rest of the system works with a pinned version.
//!
//! The temp repo is automatically cleaned up when it goes out of scope.

use crate::config::Location;
use std::process::Command;
use tracing::info;

/// Temporary bare git repo that deletes itself on drop.
///
/// We use a bare repo (no checkout) so we never download actual file content —
/// only refs and tag metadata. The `--filter=blob:none` fetch flag ensures
/// this stays lightweight even for large repositories.
struct TempBareRepo(std::path::PathBuf);

impl TempBareRepo {
    fn create() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = tempfile::Builder::new()
            .prefix("foc-devnet-tag-probe-")
            .tempdir()?;
        let path = temp_dir.into_path();
        let status = Command::new("git")
            .args(["init", "--bare"])
            .arg(&path)
            .env("GIT_TERMINAL_PROMPT", "0")
            .status()?;
        if !status.success() {
            return Err("git init --bare failed".into());
        }
        Ok(Self(path))
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

/// If `location` is `LatestTag`, resolve it to a concrete `GitTag`.
/// All other variants pass through unchanged.
///
/// Example: `LatestTag { url: "…lotus.git", branch: "master" }`
///        → `GitTag  { url: "…lotus.git", tag: "v1.35.0" }`
pub fn resolve_location(location: Location) -> Result<Location, Box<dyn std::error::Error>> {
    match location {
        Location::LatestTag { url, branch } => {
            let tag = fetch_latest_tag(&url, &branch)?;
            info!("Resolved latesttag: {} (branch {}) → {}", url, branch, tag);
            Ok(Location::GitTag { url, tag })
        }
        other => Ok(other),
    }
}

/// Fetch the newest tag on `branch` from the remote at `url`.
///
/// Steps:
///   1. `git fetch --tags --filter=blob:none <url> refs/heads/<branch>`
///      — pulls the branch ref and all tags without downloading any file blobs.
///   2. `git tag --merged <branch> --sort=-creatordate`
///      — lists tags reachable from that branch, newest first.
///   3. Take the first line → that's the latest tag.
fn fetch_latest_tag(url: &str, branch: &str) -> Result<String, Box<dyn std::error::Error>> {
    info!("Fetching newest tag on branch '{}' from {}", branch, url);

    let repo = TempBareRepo::create()?;
    let refspec = format!("refs/heads/{b}:refs/heads/{b}", b = branch);

    // Fetch branch + tags (no blobs — keeps it fast)
    let fetch = Command::new("git")
        .args(["fetch", "--tags", "--filter=blob:none", url, &refspec])
        .current_dir(repo.path())
        .env("GIT_TERMINAL_PROMPT", "0")
        .status()?;
    if !fetch.success() {
        return Err(format!("git fetch failed for {} (branch {})", url, branch).into());
    }

    // List tags reachable from the branch, newest first
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

    // First non-empty line is the newest tag
    let stdout = String::from_utf8_lossy(&tags.stdout);
    stdout
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("No tags found on branch '{}' for {}", branch, url).into())
}
