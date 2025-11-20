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

/// Main configuration structure for the foc-localnet application.
///
/// This struct contains all the settings needed to configure and run a local
/// Filecoin cluster for testing filecoin-onchain-cloud functionality. It includes
/// counts of different node types, port allocations, and locations for executables.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Number of lotus-miner nodes to run in the cluster.
    ///
    /// Lotus-miner nodes are responsible for mining operations in the Filecoin network.
    /// Default: 1
    pub lotus_miner_count: u32,

    /// Number of lotus execution nodes to run in the cluster.
    ///
    /// Lotus nodes handle the core Filecoin protocol execution, including
    /// transaction processing and block validation.
    /// Default: 1
    pub lotus_count: u32,

    /// Number of curio nodes to run in the cluster.
    ///
    /// Curio nodes provide additional services and utilities for the Filecoin network.
    /// Default: 1
    pub curio_count: u32,

    /// Number of ports to reserve for lotus-miner nodes.
    ///
    /// Each lotus-miner node requires dedicated ports for communication.
    /// This specifies how many ports per miner node.
    /// Default: 1
    pub lotus_miner_ports: u32,

    /// Number of ports to reserve for lotus execution nodes.
    ///
    /// Each lotus node requires dedicated ports for P2P communication and APIs.
    /// This specifies how many ports per lotus node.
    /// Default: 1
    pub lotus_ports: u32,

    /// Number of ports to reserve for curio nodes.
    ///
    /// Each curio node requires dedicated ports for its services.
    /// This specifies how many ports per curio node.
    /// Default: 1
    pub curio_miner_ports: u32,

    /// Location specification for the lotus executable.
    ///
    /// Defines how to obtain and run the lotus daemon executable.
    /// See [`Location`] for available options.
    pub lotus_location: Location,

    /// Location specification for the lotus-miner executable.
    ///
    /// Defines how to obtain and run the lotus-miner executable.
    /// See [`Location`] for available options.
    pub lotus_miner_location: Location,

    /// Location specification for the curio executable.
    ///
    /// Defines how to obtain and run the curio executable.
    /// See [`Location`] for available options.
    pub curio_location: Location,
}

impl Default for Config {
    /// Creates a default configuration with sensible defaults.
    ///
    /// The default configuration sets up a minimal cluster with one of each
    /// node type and assumes pre-built executables are available in standard
    /// system locations (/usr/local/bin/).
    fn default() -> Self {
        Self {
            lotus_miner_count: 1,
            lotus_count: 1,
            curio_count: 1,
            lotus_miner_ports: 1,
            lotus_ports: 1,
            curio_miner_ports: 1,
            lotus_location: Location::GitTag {
                url: "https://github.com/filecoin-project/lotus.git".to_string(),
                tag: "v1.12.0".to_string(),
            },
            lotus_miner_location: Location::GitTag {
                url: "https://github.com/filecoin-project/lotus.git".to_string(),
                tag: "v1.12.0".to_string(),
            },
            curio_location: Location::GitTag {
                url: "https://github.com/filecoin-project/curio.git".to_string(),
                tag: "v1.12.0".to_string(),
            },
        }
    }
}
