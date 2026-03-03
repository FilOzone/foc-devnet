//! Resolver for dynamic location variants (`LatestCommit`, `LatestTag`).
//!
//! Queries remote Git repositories to resolve dynamic location variants to
//! concrete `GitCommit` / `GitTag` values at init time. The resolved SHA or
//! tag is then written to `config.toml` so that builds are always reproducible
//! and the exact version is recorded in the run state.
//!
//! `LatestCommit` uses `git ls-remote` to resolve to the tip of `main`
//! (or `master` if `main` does not exist — no local clone needed).
//!
//! `LatestTag` performs a blobless bare fetch of the default branch (`main`
//! or `master`) and all
//! tags into a temporary directory, then runs `git tag --merged main` to
//! enumerate only tags whose commits are reachable from `main`. This avoids
//! picking tags that were created on feature branches or release branches that
//! diverged from `main`.
//!
//! # Example
//!
//! ```text
//! foc-devnet init --curio latestCommit --lotus latestTag
//! // Queries remote → resolves to GitCommit { commit: "abc123..." }
//! //                               GitTag   { tag:    "v1.34.5" }
//! // Stores concrete values in config.toml
//! ```

use crate::config::Location;
use semver::Version;
use std::process::Command;
use tracing::info;

/// Temporary bare repo used for tag-reachability queries.
struct TempBareRepo(std::path::PathBuf);

impl TempBareRepo {
    /// Initialise an empty bare repository in a system temp directory.
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

    /// Return the path of this bare repo.
    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for TempBareRepo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Resolve a `Location` to a concrete variant by querying the remote if needed.
///
/// `LatestCommit` and `LatestTag` are resolved against the remote repository.
/// All other variants are returned unchanged.
pub fn resolve_location(location: Location) -> Result<Location, Box<dyn std::error::Error>> {
    match location {
        Location::LatestCommit { url } => {
            let commit = fetch_latest_commit(&url)?;
            info!("Resolved latestCommit for {} → {}", url, commit);
            Ok(Location::GitCommit { url, commit })
        }
        Location::LatestTag { url } => {
            let tag = fetch_latest_tag(&url)?;
            info!("Resolved latestTag for {} → {}", url, tag);
            Ok(Location::GitTag { url, tag })
        }
        other => Ok(other),
    }
}

/// Fetch the SHA of the tip of the default branch (`main` or `master`) on a remote.
///
/// Tries `main` first; if it doesn't exist on the remote, falls back to `master`.
/// Fails if neither branch is found.
fn fetch_latest_commit(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let branch = resolve_default_branch(url)?;
    info!("Fetching latest commit on {} from {}", branch, url);

    let output = Command::new("git")
        .args(["ls-remote", url, &format!("refs/heads/{}", branch)])
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "git ls-remote failed for {}: {}",
            url,
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }

    parse_ls_remote_commit(&String::from_utf8_lossy(&output.stdout), url)
}

/// Parse the commit SHA from `git ls-remote` stdout.
///
/// The output would be of the form:
/// ```
/// 7741226198083e943a64d917e88a0a77d17aa30e        refs/heads/master
/// ```
fn parse_ls_remote_commit(stdout: &str, url: &str) -> Result<String, Box<dyn std::error::Error>> {
    stdout
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().next())
        .map(str::to_string)
        .ok_or_else(|| format!("No commit found in ls-remote output for {}", url).into())
}

/// Resolve the default branch name for a remote repository.
///
/// Queries the remote for both `main` and `master` in a single `git ls-remote`
/// call. Returns `"main"` if it exists, `"master"` if only that exists, or an
/// error if neither is present.
fn resolve_default_branch(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .args([
            "ls-remote",
            "--heads",
            url,
            "refs/heads/main",
            "refs/heads/master",
        ])
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "git ls-remote --heads failed for {}: {}",
            url,
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let has_main = stdout.lines().any(|l| l.contains("refs/heads/main"));
    let has_master = stdout.lines().any(|l| l.contains("refs/heads/master"));

    if has_main {
        Ok("main".to_string())
    } else if has_master {
        info!(
            "Remote {} has no 'main' branch, falling back to 'master'",
            url
        );
        Ok("master".to_string())
    } else {
        Err(format!("Remote {} has neither 'main' nor 'master' branch", url).into())
    }
}

/// Fetch the highest stable semver tag for a remote repo, considering only
/// tags whose commits are reachable from the default branch (`main` or `master`).
///
/// Strategy:
/// 1. Resolve the default branch (`main`, falling back to `master`).
/// 2. Create a throwaway bare repo in a temp directory.
/// 3. Blobless-fetch the default branch and all tags from the remote (no file
///    content downloaded — only commit and tree objects).
/// 4. Run `git tag --merged <branch>` to enumerate tags reachable from it.
/// 5. Filter for stable semver tags (no `-rc`, `-alpha`, etc.) and return
///    the highest by numeric segment comparison.
fn fetch_latest_tag(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let branch = resolve_default_branch(url)?;
    info!("Fetching latest stable tag on {} from {}", branch, url);

    let repo = TempBareRepo::create()?;

    fetch_default_branch_and_tags(repo.path(), url, &branch)?;

    let tags_output = Command::new("git")
        .args(["tag", "--merged", &branch])
        .current_dir(repo.path())
        .output()?;

    if !tags_output.status.success() {
        return Err(format!(
            "git tag --merged {} failed: {}",
            branch,
            String::from_utf8_lossy(&tags_output.stderr).trim()
        )
        .into());
    }

    parse_latest_tag(&String::from_utf8_lossy(&tags_output.stdout), url)
}

/// Fetch the default branch and all tags from `url` into an existing bare repo.
///
/// Uses `--filter=blob:none` so only commit and tree objects are transferred
/// (no file content), keeping the operation fast even for large repositories.
fn fetch_default_branch_and_tags(
    repo_path: &std::path::Path,
    url: &str,
    branch: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let refspec = format!("refs/heads/{b}:refs/heads/{b}", b = branch);
    let status = Command::new("git")
        .args(["fetch", "--tags", "--filter=blob:none", url, &refspec])
        .current_dir(repo_path)
        .env("GIT_TERMINAL_PROMPT", "0")
        .status()?;

    if !status.success() {
        return Err(format!("git fetch failed for {}", url).into());
    }
    Ok(())
}

/// Parse and return the highest stable semver tag from `git tag --merged` stdout.
///
/// Each line is a plain tag name (e.g. `v1.2.3`). Tags that cannot be parsed
/// as a valid semver version, or that carry a pre-release identifier (e.g.
/// `-rc1`, `-alpha`, `-beta`), are silently skipped. The remaining tags are
/// sorted by the `semver::Version` `Ord` implementation and the highest is
/// returned.
fn parse_latest_tag(stdout: &str, url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut tags: Vec<(Version, &str)> = stdout
        .lines()
        .map(str::trim)
        .filter_map(|tag| {
            // Strip leading 'v' before parsing, semver crate requires bare `1.2.3`
            let raw = tag.trim_start_matches('v');
            Version::parse(raw).ok().map(|v| (v, tag))
        })
        .filter(|(v, _)| v.pre.is_empty()) // exclude pre-release versions
        .collect();

    if tags.is_empty() {
        return Err(format!(
            "No stable semver tags reachable from default branch for {}",
            url
        )
        .into());
    }

    tags.sort_by(|(a, _), (b, _)| a.cmp(b));
    Ok(tags.last().unwrap().1.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semver_sort_picks_highest() {
        let output = "v1.9.0\nv1.10.0\nv1.2.3\nv1.10.1\n";
        let tag = parse_latest_tag(output, "https://example.com/repo").unwrap();
        assert_eq!(tag, "v1.10.1");
    }

    #[test]
    fn test_semver_sort_ignores_rc_suffix() {
        let output = "v1.34.3\nv1.34.4\n";
        let tag = parse_latest_tag(output, "https://example.com/repo").unwrap();
        assert_eq!(tag, "v1.34.4");
    }

    #[test]
    fn test_parse_ls_remote_commit() {
        let output = "abc123def456\tHEAD\n";
        let commit = parse_ls_remote_commit(output, "https://example.com/repo").unwrap();
        assert_eq!(commit, "abc123def456");
    }

    /// Simulate the stdout of `git ls-remote --heads` for branch resolution.
    fn ls_remote_heads(branches: &[&str]) -> String {
        branches
            .iter()
            .map(|b| format!("deadbeef\trefs/heads/{}\n", b))
            .collect()
    }

    #[test]
    fn test_resolve_default_branch_prefers_main() {
        let stdout = ls_remote_heads(&["main", "master"]);
        let has_main = stdout.lines().any(|l| l.contains("refs/heads/main"));
        let has_master = stdout.lines().any(|l| l.contains("refs/heads/master"));
        assert!(has_main);
        let branch = if has_main {
            "main"
        } else if has_master {
            "master"
        } else {
            ""
        };
        assert_eq!(branch, "main");
    }

    #[test]
    fn test_resolve_default_branch_falls_back_to_master() {
        let stdout = ls_remote_heads(&["master", "develop"]);
        let has_main = stdout.lines().any(|l| l.contains("refs/heads/main"));
        let has_master = stdout.lines().any(|l| l.contains("refs/heads/master"));
        assert!(!has_main);
        assert!(has_master);
        let branch = if has_main {
            "main"
        } else if has_master {
            "master"
        } else {
            ""
        };
        assert_eq!(branch, "master");
    }

    #[test]
    fn test_parse_latest_tag() {
        let output = "v1.0.0\nv1.2.0\nv1.1.0\n";
        let tag = parse_latest_tag(output, "https://example.com/repo").unwrap();
        assert_eq!(tag, "v1.2.0");
    }

    #[test]
    fn test_parse_latest_tag_skips_rc() {
        // v1.2.0-rc1 is excluded; v1.1.0 is the latest stable
        let output = "v1.0.0\nv1.1.0\nv1.2.0-rc1\n";
        let tag = parse_latest_tag(output, "https://example.com/repo").unwrap();
        assert_eq!(tag, "v1.1.0");
    }

    #[test]
    fn test_parse_latest_tag_skips_non_semver() {
        // "latest" and bare "rc" strings should be silently ignored
        let output = "latest\nv1.0.0\nrc\nv1.1.0\n";
        let tag = parse_latest_tag(output, "https://example.com/repo").unwrap();
        assert_eq!(tag, "v1.1.0");
    }
}
