//! External API module for DevNet information export.
//!
//! This module provides versioned, schema-based JSON output that can be consumed
//! by external tools and scripts (e.g., JavaScript, Python) to interact with
//! the running DevNet.

mod devnet_info;
mod export;

pub use devnet_info::{
    ContractsInfo, CurioInfo, DatabaseInfo, DevnetInfoV2, LotusInfo, LotusMinerInfo, UserInfo,
    VersionedDevnetInfo,
};
pub use export::export_devnet_info;

/// Current schema version for DevNet info export. v2 replaced the per-SP
/// `yugabyte` block with a `database` block (Postgres + Scylla).
pub const DEVNET_INFO_SCHEMA_VERSION: u32 = 2;
