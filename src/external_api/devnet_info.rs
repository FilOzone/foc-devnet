//! DevNet information data structures.
//!
//! Contains the versioned schema for exporting DevNet state to external consumers.

use serde::{Deserialize, Serialize};

/// Versioned wrapper for DevNet information.
///
/// This struct provides a stable interface for external consumers. The `version`
/// field indicates the schema version, allowing consumers to handle migrations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedDevnetInfo {
    /// Schema version number. Increment when breaking changes are made.
    pub version: u32,
    /// The actual DevNet information payload.
    pub info: DevnetInfoV1,
}

/// DevNet information schema version 1.
///
/// Contains all relevant information about a running DevNet instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevnetInfoV1 {
    /// Unique identifier for this run (e.g., "26jan20-1622_GoofyNubs")
    pub run_id: String,
    /// ISO 8601 formatted start time
    pub start_time: String,
    /// Time taken to start the system (e.g., "539.04s")
    pub startup_duration: String,
    /// User accounts available for testing
    pub users: Vec<UserInfo>,
    /// Deployed contract addresses
    pub contracts: ContractsInfo,
    /// Lotus node information
    pub lotus: LotusInfo,
    /// Lotus miner information
    pub lotus_miner: LotusMinerInfo,
    /// PDP service providers
    pub pdp_sps: Vec<CurioInfo>,
}

/// Information about a user account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    /// User identifier (e.g., "USER_1")
    pub name: String,
    /// Ethereum-compatible address (0x...)
    pub evm_addr: String,
    /// Native Filecoin address (t410f...)
    pub native_addr: String,
    /// Private key in hex format (without 0x prefix)
    pub private_key_hex: String,
}

/// Deployed contract addresses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractsInfo {
    /// Multicall3 contract address
    pub multicall3_addr: String,
    /// MockUSDFC token contract address
    pub mockusdfc_addr: String,
    /// Filecoin Warm Storage Service proxy address
    pub fwss_service_proxy_addr: String,
    /// Filecoin Warm Storage Service state view address
    pub fwss_state_view_addr: String,
    /// Filecoin Warm Storage Service implementation address
    pub fwss_impl_addr: String,
    /// PDP Verifier proxy address
    pub pdp_verifier_proxy_addr: String,
    /// PDP Verifier implementation address
    pub pdp_verifier_impl_addr: String,
    /// Service Provider Registry proxy address
    pub service_provider_registry_proxy_addr: String,
    /// Service Provider Registry implementation address
    pub service_provider_registry_impl_addr: String,
    /// FilecoinPay V1 contract address
    pub filecoin_pay_v1_addr: String,
    /// Endorsements contract address
    pub endorsements_addr: String,
}

/// Lotus node information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LotusInfo {
    /// RPC URL exposed to the host
    pub host_rpc_url: String,
    /// Docker container ID
    pub container_id: String,
    /// Docker container name
    pub container_name: String,
}

/// Lotus miner information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LotusMinerInfo {
    /// Docker container ID
    pub container_id: String,
    /// Docker container name
    pub container_name: String,
    /// API port
    pub api_port: u16,
}

/// Curio service provider information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurioInfo {
    /// Provider ID (1-indexed)
    pub provider_id: u32,
    /// Ethereum address of the provider
    pub eth_addr: String,
    /// Native Filecoin address
    pub native_addr: String,
    /// PDP service URL accessible from host
    pub pdp_service_url: String,
    /// Docker container ID
    pub container_id: String,
    /// Docker container name
    pub container_name: String,
    /// YugabyteDB information for this provider
    pub yugabyte: YugabyteInfo,
}

/// YugabyteDB connection information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YugabyteInfo {
    /// Web UI URL exposed to host
    pub web_ui_url: String,
    /// Master RPC port
    pub master_rpc_port: u16,
    /// YSQL port for Postgres-compatible connections
    pub ysql_port: u16,
}
