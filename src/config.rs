//! Configuration module for foc-devnet.
//!
//! This module defines the configuration structures used to manage the local
//! Filecoin on-chain cloud cluster. It includes settings for node counts,
//! port allocations, and executable locations for various components.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

/// Represents the location of an executable or source code for a component.
///
/// This enum allows specifying how to obtain and run different Filecoin-related
/// executables (lotus, lotus-miner, curio) in various deployment scenarios.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Location {
    /// Use a local directory containing source code that needs to be built.
    ///
    /// The `dir` field should point to a directory with the source code.
    /// The application will handle building the executable from this source.
    LocalSource { dir: String },

    /// Fetch source code from a Git repository at a specific commit.
    ///
    /// The `url` field is the Git repository URL, and `commit` is the specific
    /// commit hash to check out. The application will clone the repo and
    /// build the executable from this commit.
    GitCommit { url: String, commit: String },

    /// Fetch source code from a Git repository at a specific tag.
    ///
    /// The `url` field is the Git repository URL, and `tag` is the specific
    /// tag (e.g., "v1.2.3") to check out. This is useful for stable releases.
    GitTag { url: String, tag: String },

    /// Fetch source code from a Git repository at a specific branch.
    ///
    /// The `url` field is the Git repository URL, and `branch` is the specific
    /// branch (e.g., "main", "develop") to check out.
    GitBranch { url: String, branch: String },
}

static DEPENDENCY_MANIFEST: OnceLock<DependencyManifest> = OnceLock::new();

#[derive(Debug, Deserialize)]
struct DependencyManifest {
    schema_version: u32,
    dependencies: HashMap<String, DependencyDefinition>,
}

#[derive(Debug, Deserialize)]
struct DependencyDefinition {
    git: String,
    default: DependencySelection,
}

#[derive(Debug, Deserialize)]
struct DependencySelection {
    tag: Option<String>,
    commit: Option<String>,
    branch: Option<String>,
    bundled: Option<bool>,
}

fn dependency_manifest() -> &'static DependencyManifest {
    DEPENDENCY_MANIFEST.get_or_init(|| {
        let manifest: DependencyManifest = toml::from_str(include_str!("../dependencies.toml"))
            .expect("embedded dependencies.toml must be valid");
        assert_eq!(
            manifest.schema_version, 1,
            "embedded dependencies.toml schema_version must be 1"
        );
        manifest
    })
}

fn dependency_definition(name: &str) -> &'static DependencyDefinition {
    dependency_manifest()
        .dependencies
        .get(name)
        .unwrap_or_else(|| panic!("embedded dependencies.toml is missing {name}"))
}

fn default_dependency_location(name: &str) -> Option<Location> {
    let definition = dependency_definition(name);
    let selection = &definition.default;

    if selection.bundled == Some(true) {
        return None;
    }
    if let Some(tag) = &selection.tag {
        return Some(Location::GitTag {
            url: definition.git.clone(),
            tag: tag.clone(),
        });
    }
    if let Some(commit) = &selection.commit {
        return Some(Location::GitCommit {
            url: definition.git.clone(),
            commit: commit.clone(),
        });
    }
    if let Some(branch) = &selection.branch {
        return Some(Location::GitBranch {
            url: definition.git.clone(),
            branch: branch.clone(),
        });
    }
    panic!("embedded dependencies.toml default for {name} must be a git location or bundled")
}

pub fn default_dependency_repository(name: &str) -> String {
    dependency_definition(name).git.clone()
}

impl Location {
    /// Given a url and a selector, finds the latest tag given that selector.
    ///
    /// Runs `git ls-remote --tags --sort=-version:refname <url> <selector>` and
    /// returns the first tag name from the output (i.e. the lexicographically newest
    /// version-sorted tag that matches `selector`).
    ///
    /// Example: `resolveLatestTag("https://github.com/foo/bar.git", "v*")` might
    /// return `"v1.2.3"`.
    fn resolve_latest_tag(url: &str, selector: &str) -> Result<String, String> {
        let stdout = {
            #[cfg(not(test))]
            {
                let output = std::process::Command::new("git")
                    .args([
                        "ls-remote",
                        "--tags",
                        "--sort=-version:refname",
                        url,
                        selector,
                    ])
                    .output()
                    .map_err(|e| format!("Failed to run git ls-remote: {}", e))?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(format!("git ls-remote failed: {}", stderr.trim()));
                }

                String::from_utf8_lossy(&output.stdout).into_owned()
            }

            #[cfg(test)]
            {
                String::from("0000000000000 refs/tags/v1.0.0")
            }
        };

        let first_line = stdout
            .lines()
            .next()
            .ok_or_else(|| format!("No tags found for selector '{}' at '{}'", selector, url))?;

        // Output format: "<hash>\trefs/tags/<tag>"
        let tag_ref = first_line
            .split_whitespace()
            .nth(1)
            .ok_or_else(|| format!("Unexpected git ls-remote output: '{}'", first_line))?;

        tag_ref
            .strip_prefix("refs/tags/")
            .map(|t| t.to_string())
            .ok_or_else(|| format!("Unexpected tag ref format: '{}'", tag_ref))
    }

    /// Canonicalizes the location from a set of following variants:
    ///
    /// - `latesttag`                — newest tag matching `*` (uses default URL)
    /// - `latesttag:<selector>`     — newest tag matching selector (e.g. `latesttag:pdp/v*`)
    /// - `latesttag:<url>:<selector>` — newest tag matching selector at a custom URL
    /// - `gittag:<tag>`             — (uses default URL)
    /// - `gitcommit:<commit>`       — (uses default URL)
    /// - `gitbranch:<branch>`       — (uses default URL)
    /// - `local:<dir>`
    /// - `gittag:<url>:<tag>`
    /// - `gitcommit:<url>:<commit>`
    /// - `gitbranch:<url>:<branch>`
    ///
    /// to a smaller subset:
    ///
    /// - `local:<dir>`              -> `("local",      "",     "dir")`
    /// - `gittag:<url>:<tag>`       -> `("gittag",     "url",  "tag")`
    /// - `gitcommit:<url>:<commit>` -> `("gitcommit",  "url",  "commit")`
    /// - `gitbranch:<url>:<branch>` -> `("gitbranch",  "url",  "branch")`
    fn canonicalize_location(
        location: &str,
        default_url: &str,
    ) -> Result<(String, String, String), String> {
        // Special case: bare "latesttag" with no selector — implicitly matches all tags.
        if location == "latesttag" {
            let tag = Self::resolve_latest_tag(default_url, "*")?;
            return Ok(("gittag".into(), default_url.into(), tag));
        }

        // We need to do this setup in two steps since otherwise
        // "gittag:https://github.com/orgs/repo:v1.2.0" would not be parseable
        // and split the <url> string itself in two parts
        let (location_type, remaining) = location.split_once(':').ok_or_else(|| {
            format!(
                "Invalid location format: '{}'. Expected \
                 'latesttag:<selector>', or 'gittag/gitcommit/gitbranch/local:...'",
                location
            )
        })?;

        // If the remaining part contains another ':', the portion before the last ':'
        // is the URL and everything after is the value (handles HTTPS URLs with colons).
        let (url, selector) = if let Some(colon_pos) = remaining.rfind(':') {
            (&remaining[..colon_pos], &remaining[colon_pos + 1..])
        } else {
            (default_url, remaining)
        };

        // Special case: latesttag resolution into GitTag
        if location_type == "latesttag" {
            let tag = Self::resolve_latest_tag(url, selector)?;
            return Ok(("gittag".into(), url.into(), tag));
        }

        Ok((location_type.into(), url.into(), selector.into()))
    }

    /// Parse a location string in the format "type:value" or "type:url:value".
    /// May attempt to resolve `latesttag` if provided by reaching over the internet.
    ///
    /// Supported formats:
    /// - `latesttag`                       — newest tag (uses default URL, matches `*`)
    /// - `latesttag:<selector>`            — newest tag matching selector (e.g. `latesttag:pdp/v*`)
    /// - `latesttag:<url>:<selector>`      — newest tag matching selector at a custom URL
    /// - `gittag:<tag>`                    — (uses default URL)
    /// - `gitcommit:<commit>`              — (uses default URL)
    /// - `gitbranch:<branch>`              — (uses default URL)
    /// - `local:<dir>`
    /// - `gittag:<url>:<tag>`
    /// - `gitcommit:<url>:<commit>`
    /// - `gitbranch:<url>:<branch>`
    pub fn resolve_with_default(location: &str, default_url: &str) -> Result<Self, String> {
        let canonical_location = Self::canonicalize_location(location, default_url)?;
        let (loc_type, url, selector) = canonical_location;

        match loc_type.as_ref() {
            "local" => Ok(Location::LocalSource { dir: selector }),
            "gittag" => Ok(Location::GitTag { url, tag: selector }),
            "gitcommit" => Ok(Location::GitCommit {
                url,
                commit: selector,
            }),
            "gitbranch" => Ok(Location::GitBranch {
                url,
                branch: selector,
            }),
            _ => Err(format!(
                "Unknown location type: {}. Supported types: local, gittag, gitcommit, gitbranch",
                loc_type
            )),
        }
    }
}

/// Main configuration structure for the foc-devnet application.
///
/// This struct contains all the settings needed to configure and run a local
/// Filecoin cluster for testing filecoin-onchain-cloud functionality. It includes
/// counts of different node types, port allocations, and locations for executables.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Starting port number for the contiguous port range.
    ///
    /// All ports used by the devnet will be dynamically allocated from a contiguous
    /// range starting at this port. This ensures no port conflicts and allows
    /// easy firewall configuration.
    /// Default: 5700
    pub port_range_start: u16,

    /// Number of ports in the contiguous port range.
    ///
    /// This defines the size of the port range available for allocation.
    /// For example, with port_range_start=5700 and port_range_count=300,
    /// ports 5700-5999 are reserved for the devnet.
    /// Default: 100
    pub port_range_count: u16,

    /// Location specification for the lotus executable.
    ///
    /// Defines how to obtain and run the lotus daemon executable.
    /// See [`Location`] for available options.
    pub lotus: Location,

    /// Location specification for the curio executable.
    ///
    /// Defines how to obtain and run the curio executable.
    /// See [`Location`] for available options.
    pub curio: Location,

    /// Location specification for the filecoin-services repository.
    ///
    /// Defines how to obtain the filecoin-services code, which contains
    /// the FOC (Filecoin Onchain Contracts) deployment scripts needed by Curio.
    /// See [`Location`] for available options.
    pub filecoin_services: Location,

    /// Optional location specification for the PDP repository.
    ///
    /// When unset, foc-devnet uses the PDP submodule bundled with
    /// filecoin-services. When set, this checkout is mounted over
    /// service_contracts/lib/pdp during FOC contract deployment.
    /// See [`Location`] for available options.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pdp: Option<Location>,

    /// Location specification for the multicall3 repository.
    ///
    /// Defines how to obtain the multicall3 code, which provides the
    /// Multicall3 contract for batching multiple calls in a single transaction.
    /// See [`Location`] for available options.
    pub multicall3: Location,

    /// Number of approved PDP service providers.
    ///
    /// This is the total number of Curio SPs that will be registered and approved
    /// in the service provider registry. These SPs can accept storage deals.
    /// Must satisfy: ENDORSED_PDP_SP_COUNT <= APPROVED_PDP_SP_COUNT <= ACTIVE_PDP_SP_COUNT <= MAX_PDP_SP_COUNT
    /// Default: 1
    pub approved_pdp_sp_count: usize,

    /// Number of endorsed PDP service providers.
    ///
    /// This is the number of approved SPs that will be endorsed in the endorsements contract.
    /// Endorsed providers are a privileged subset of approved providers.
    /// Must satisfy: ENDORSED_PDP_SP_COUNT <= APPROVED_PDP_SP_COUNT <= ACTIVE_PDP_SP_COUNT <= MAX_PDP_SP_COUNT
    /// Default: 1
    pub endorsed_pdp_sp_count: usize,

    /// Number of active PDP service providers.
    ///
    /// This is the total number of Curio SPs that will actually be started/running.
    /// Some may be approved, some may not (for testing unapproved SP scenarios).
    /// Total miners = 1 (lotus-miner) + ACTIVE_PDP_SP_COUNT (curio SPs)
    /// Must satisfy: ENDORSED_PDP_SP_COUNT <= APPROVED_PDP_SP_COUNT <= ACTIVE_PDP_SP_COUNT <= MAX_PDP_SP_COUNT
    /// Default: 1
    pub active_pdp_sp_count: usize,
}

impl Default for Config {
    /// Creates a default configuration with sensible defaults.
    ///
    /// The default configuration sets up a minimal cluster with one of each
    /// node type and assumes pre-built executables are available in standard
    /// system locations (/usr/local/bin/).
    ///
    /// The defaults should always use `GitCommit` or `GitTag` locations to ensure
    /// reproducibility.
    fn default() -> Self {
        Self {
            port_range_start: 5700,
            port_range_count: 100,
            lotus: default_dependency_location("lotus")
                .expect("lotus default dependency must not be bundled"),
            curio: default_dependency_location("curio")
                .expect("curio default dependency must not be bundled"),
            filecoin_services: default_dependency_location("filecoin-services")
                .expect("filecoin-services default dependency must not be bundled"),
            pdp: default_dependency_location("pdp"),
            multicall3: default_dependency_location("multicall3")
                .expect("multicall3 default dependency must not be bundled"),
            approved_pdp_sp_count: 2,
            endorsed_pdp_sp_count: 1,
            active_pdp_sp_count: 2,
        }
    }
}

impl Config {
    /// Validate configuration values.
    ///
    /// Ensures that:
    /// - ENDORSED_PDP_SP_COUNT <= APPROVED_PDP_SP_COUNT <= ACTIVE_PDP_SP_COUNT <= MAX_PDP_SP_COUNT
    pub fn validate(&self) -> Result<(), String> {
        const MAX_PDP_SP_COUNT: usize = crate::constants::MAX_PDP_SP_COUNT;

        if self.endorsed_pdp_sp_count > self.approved_pdp_sp_count {
            return Err(format!(
                "endorsed_pdp_sp_count ({}) cannot exceed approved_pdp_sp_count ({})",
                self.endorsed_pdp_sp_count, self.approved_pdp_sp_count
            ));
        }

        if self.approved_pdp_sp_count > self.active_pdp_sp_count {
            return Err(format!(
                "approved_pdp_sp_count ({}) cannot exceed active_pdp_sp_count ({})",
                self.approved_pdp_sp_count, self.active_pdp_sp_count
            ));
        }

        if self.active_pdp_sp_count > MAX_PDP_SP_COUNT {
            return Err(format!(
                "active_pdp_sp_count ({}) cannot exceed MAX_PDP_SP_COUNT ({})",
                self.active_pdp_sp_count, MAX_PDP_SP_COUNT
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{default_dependency_repository, Config, Location};

    const DEFAULT_URL: &str = "https://github.com/default/repo.git";

    fn canonicalize(s: &str) -> Result<(String, String, String), String> {
        Location::canonicalize_location(s, DEFAULT_URL)
    }

    // --- happy-path tests ---

    #[test]
    fn gittag_short_uses_default_url() {
        assert_eq!(
            canonicalize("gittag:v1.2.3").unwrap(),
            ("gittag".into(), DEFAULT_URL.into(), "v1.2.3".into())
        );
    }

    #[test]
    fn gittag_explicit_https_url() {
        assert_eq!(
            canonicalize("gittag:https://github.com/foo/bar.git:v1.2.3").unwrap(),
            (
                "gittag".into(),
                "https://github.com/foo/bar.git".into(),
                "v1.2.3".into()
            )
        );
    }

    #[test]
    fn gitcommit_short_uses_default_url() {
        assert_eq!(
            canonicalize("gitcommit:abc123def456").unwrap(),
            (
                "gitcommit".into(),
                DEFAULT_URL.into(),
                "abc123def456".into()
            )
        );
    }

    #[test]
    fn gitcommit_explicit_https_url() {
        assert_eq!(
            canonicalize("gitcommit:https://github.com/foo/bar.git:abc123def456").unwrap(),
            (
                "gitcommit".into(),
                "https://github.com/foo/bar.git".into(),
                "abc123def456".into()
            )
        );
    }

    #[test]
    fn gitbranch_short_uses_default_url() {
        assert_eq!(
            canonicalize("gitbranch:main").unwrap(),
            ("gitbranch".into(), DEFAULT_URL.into(), "main".into())
        );
    }

    #[test]
    fn gitbranch_explicit_https_url() {
        assert_eq!(
            canonicalize("gitbranch:https://github.com/foo/bar.git:feature/my-branch").unwrap(),
            (
                "gitbranch".into(),
                "https://github.com/foo/bar.git".into(),
                "feature/my-branch".into()
            )
        );
    }

    #[test]
    fn local_dir() {
        assert_eq!(
            canonicalize("local:/home/user/my-project").unwrap(),
            (
                "local".into(),
                DEFAULT_URL.into(),
                "/home/user/my-project".into()
            )
        );
    }

    #[test]
    fn latesttag_with_url() {
        assert_eq!(
            canonicalize("latesttag:https://github.com/randomorg/randomrepo:v*").unwrap(),
            (
                "gittag".into(),
                "https://github.com/randomorg/randomrepo".into(),
                "v1.0.0".into()
            )
        );
    }

    #[test]
    fn latesttag_without_url() {
        assert_eq!(
            canonicalize("latesttag:v*").unwrap(),
            ("gittag".into(), DEFAULT_URL.into(), "v1.0.0".into())
        );
    }

    #[test]
    fn latesttag_without_url_without_selector() {
        assert_eq!(
            canonicalize("latesttag").unwrap(),
            ("gittag".into(), DEFAULT_URL.into(), "v1.0.0".into())
        );
    }

    // --- error-path tests ---

    #[test]
    fn missing_colon_returns_error() {
        assert!(canonicalize("gittag").is_err());
    }

    #[test]
    fn empty_string_returns_error() {
        assert!(canonicalize("").is_err());
    }

    #[test]
    fn default_config_omits_optional_pdp_and_parses_without_it() {
        let serialized = toml::to_string(&Config::default()).unwrap();

        assert!(!serialized.contains("[pdp"));
        assert!(!serialized.contains("pdp ="));
        let parsed: Config = toml::from_str(&serialized).unwrap();
        assert!(parsed.pdp.is_none());
    }

    #[test]
    fn default_config_uses_manifest_dependency_defaults() {
        let config = Config::default();

        assert_eq!(
            config.lotus,
            Location::GitTag {
                url: default_dependency_repository("lotus"),
                tag: "v1.36.2".to_string(),
            }
        );
        assert_eq!(
            config.curio,
            Location::GitTag {
                url: default_dependency_repository("curio"),
                tag: "v1.28.3".to_string(),
            }
        );
        assert_eq!(
            config.filecoin_services,
            Location::GitTag {
                url: default_dependency_repository("filecoin-services"),
                tag: "v1.3.0".to_string(),
            }
        );
        assert_eq!(
            config.multicall3,
            Location::GitTag {
                url: default_dependency_repository("multicall3"),
                tag: "v3.1.0".to_string(),
            }
        );
    }

    #[test]
    fn config_serializes_configured_pdp() {
        let config = Config {
            pdp: Some(Location::GitTag {
                url: "https://github.com/FilOzone/pdp.git".to_string(),
                tag: "v3.4.0".to_string(),
            }),
            ..Default::default()
        };

        let serialized = toml::to_string(&config).unwrap();

        assert!(serialized.contains("pdp"));
        assert!(serialized.contains("https://github.com/FilOzone/pdp.git"));
    }
}
