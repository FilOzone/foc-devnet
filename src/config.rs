//! Configuration module for foc-localnet.
//!
//! This module defines the configuration structures used to manage the local
//! Filecoin on-chain cloud cluster. It includes settings for node counts,
//! port allocations, and executable locations for various components.

use serde::{Deserialize, Serialize};

/// Represents the location of an executable or source code for a component.
///
/// This enum allows specifying how to obtain and run different Filecoin-related
/// executables (lotus, lotus-miner, curio) in various deployment scenarios.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Location {
    /// Parse a location string in the format "type:value" or "type:url:value"
    ///
    /// Supported formats:
    /// - "gittag:tag" (uses default URL)
    /// - "gitcommit:commit" (uses default URL)
    /// - "gitbranch:branch" (uses default URL)
    /// - "local:dir"
    /// - "gittag:url:tag"
    /// - "gitcommit:url:commit"
    /// - "gitbranch:url:branch"
    ///
    /// Where url can contain colons (e.g., https://github.com/repo.git)
    pub fn parse_with_default(s: &str, default_url: &str) -> Result<Self, String> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() < 2 {
            return Err(format!(
                "Invalid location format: {}. Expected 'type:value' or 'type:url:value'",
                s
            ));
        }

        let location_type = parts[0];
        let remaining = &parts[1..].join(":");

        match location_type {
            "local" => Ok(Location::LocalSource {
                dir: remaining.to_string(),
            }),
            "gittag" | "gitcommit" | "gitbranch" => {
                // Check if remaining contains ':' (indicating url:value format)
                if let Some(colon_pos) = remaining.rfind(':') {
                    let url = &remaining[..colon_pos];
                    let value = &remaining[colon_pos + 1..];
                    match location_type {
                        "gittag" => Ok(Location::GitTag {
                            url: url.to_string(),
                            tag: value.to_string(),
                        }),
                        "gitcommit" => Ok(Location::GitCommit {
                            url: url.to_string(),
                            commit: value.to_string(),
                        }),
                        "gitbranch" => Ok(Location::GitBranch {
                            url: url.to_string(),
                            branch: value.to_string(),
                        }),
                        _ => unreachable!(),
                    }
                } else {
                    // No colon, so remaining is just the value, use default URL
                    match location_type {
                        "gittag" => Ok(Location::GitTag {
                            url: default_url.to_string(),
                            tag: remaining.to_string(),
                        }),
                        "gitcommit" => Ok(Location::GitCommit {
                            url: default_url.to_string(),
                            commit: remaining.to_string(),
                        }),
                        "gitbranch" => Ok(Location::GitBranch {
                            url: default_url.to_string(),
                            branch: remaining.to_string(),
                        }),
                        _ => unreachable!(),
                    }
                }
            }
            _ => Err(format!(
                "Unknown location type: {}. Supported types: local, gittag, gitcommit, gitbranch",
                location_type
            )),
        }
    }
}

/// Main configuration structure for the foc-localnet application.
///
/// This struct contains all the settings needed to configure and run a local
/// Filecoin cluster for testing filecoin-onchain-cloud functionality. It includes
/// counts of different node types, port allocations, and locations for executables.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Starting port number for the contiguous port range.
    ///
    /// All ports used by the localnet will be dynamically allocated from a contiguous
    /// range starting at this port. This ensures no port conflicts and allows
    /// easy firewall configuration.
    /// Default: 5700
    pub port_range_start: u16,

    /// Number of ports in the contiguous port range.
    ///
    /// This defines the size of the port range available for allocation.
    /// For example, with port_range_start=5700 and port_range_count=300,
    /// ports 5700-5999 are reserved for the localnet.
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

    /// Location specification for the multicall3 repository.
    ///
    /// Defines how to obtain the multicall3 code, which provides the
    /// Multicall3 contract for batching multiple calls in a single transaction.
    /// See [`Location`] for available options.
    pub multicall3: Location,

    /// Location specification for the synapse-sdk repository.
    ///
    /// Defines how to obtain the synapse-sdk code, which provides
    /// SDK functionality for Filecoin integration.
    /// See [`Location`] for available options.
    pub synapse_sdk: Location,

    /// URL to download Yugabyte database tarball.
    ///
    /// This is the direct link to the Yugabyte tarball required for running curio.
    /// Default: https://software.yugabyte.com/releases/2.25.1.0/yugabyte-2.25.1.0-b381-linux-x86_64.tar.gz
    pub yugabyte_download_url: String,
}

impl Default for Config {
    /// Creates a default configuration with sensible defaults.
    ///
    /// The default configuration sets up a minimal cluster with one of each
    /// node type and assumes pre-built executables are available in standard
    /// system locations (/usr/local/bin/).
    fn default() -> Self {
        Self {
            port_range_start: 5700,
            port_range_count: 100,
            lotus: Location::GitTag {
                url: "https://github.com/filecoin-project/lotus.git".to_string(),
                tag: "v1.34.0".to_string(),
            },
            curio: Location::GitBranch {
                url: "https://github.com/filecoin-project/curio.git".to_string(),
                branch: "pdpv0".to_string(),
            },
            filecoin_services: Location::GitTag {
                url: "https://github.com/FilOzone/filecoin-services.git".to_string(),
                tag: "v1.0.0".to_string(),
            },
            multicall3: Location::GitBranch {
                url: "https://github.com/mds1/multicall3.git".to_string(),
                branch: "main".to_string(),
            },
            synapse_sdk: Location::GitTag {
                url: "git@github.com:FilOzone/synapse-sdk.git".to_string(),
                tag: "synapse-sdk-v0.36.1".to_string(),
            },
            yugabyte_download_url: "https://software.yugabyte.com/releases/2.25.1.0/yugabyte-2.25.1.0-b381-linux-x86_64.tar.gz".to_string(),
        }
    }
}
