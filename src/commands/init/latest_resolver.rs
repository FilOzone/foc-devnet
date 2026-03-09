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

/// Prefix for the temporary bare-repo directory used during tag probing.
const TEMP_DIR_PREFIX: &str = "foc-devnet-tag-probe-";

/// Temporary bare git repo that deletes itself on drop.
///
/// We use a bare repo (no checkout) so we never download actual file content —
/// only refs and tag metadata. The `--filter=blob:none` fetch flag ensures
/// this stays lightweight even for large repositories.
struct TempBareRepo(std::path::PathBuf);

impl TempBareRepo {
    fn create() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = tempfile::Builder::new().prefix(TEMP_DIR_PREFIX).tempdir()?;
        let path = temp_dir.keep();
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
/// Orchestrates: repo creation → ref fetch → tag listing → tag selection.
fn fetch_latest_tag(url: &str, branch: &str) -> Result<String, Box<dyn std::error::Error>> {
    info!("Fetching newest tag on branch '{}' from {}", branch, url);
    let repo = TempBareRepo::create()?;
    fetch_refs(&repo, url, branch)?;
    let stdout = list_merged_tags(&repo, branch)?;
    pick_first_tag(&stdout, branch, url)
}

/// Run `git fetch --tags --filter=blob:none` to pull the branch ref and all
/// tags without downloading any file blobs.
fn fetch_refs(
    repo: &TempBareRepo,
    url: &str,
    branch: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let refspec = format!("refs/heads/{b}:refs/heads/{b}", b = branch);
    let status = Command::new("git")
        .args(["fetch", "--tags", "--filter=blob:none", url, &refspec])
        .current_dir(repo.path())
        .env("GIT_TERMINAL_PROMPT", "0")
        .status()?;
    if !status.success() {
        return Err(format!("git fetch failed for {} (branch {})", url, branch).into());
    }
    Ok(())
}

/// Run `git tag --merged <branch> --sort=-creatordate` and return stdout.
///
/// The output is a newline-separated list of tag names reachable from `branch`,
/// ordered newest-first by creator date.
fn list_merged_tags(
    repo: &TempBareRepo,
    branch: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .args(["tag", "--merged", branch, "--sort=-creatordate"])
        .current_dir(repo.path())
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "git tag --merged {} failed: {}",
            branch,
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Return the first non-empty line from `stdout` (the newest tag).
///
/// Example: given `"v1.35.0\nv1.34.0\n"` this returns `"v1.35.0"`.
fn pick_first_tag(
    stdout: &str,
    branch: &str,
    url: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    stdout
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("No tags found on branch '{}' for {}", branch, url).into())
}
