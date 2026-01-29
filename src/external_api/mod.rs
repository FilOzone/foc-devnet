//! External API module for DevNet information export.
//!
//! This module provides versioned, schema-based JSON output that can be consumed
//! by external tools and scripts (e.g., JavaScript, Python) to interact with
//! the running DevNet.

mod devnet_info;
mod export;

pub use devnet_info::{
    ContractsInfo, CurioInfo, DevnetInfoV1, LotusInfo, LotusMinerInfo, UserInfo,
    VersionedDevnetInfo, YugabyteInfo,
};
pub use export::export_devnet_info;

/// Current schema version for DevNet info export.
pub const DEVNET_INFO_SCHEMA_VERSION: u32 = 1;
