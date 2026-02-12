//! Constants for Curio step configuration.

/// Curio Web RPC API port
pub const CURIO_WEB_RPC_PORT: u16 = 4701;

/// Curio storage path inside container (fast)
pub const CURIO_FAST_STORAGE_PATH: &str = "/home/foc-user/curio/fast-storage";
/// Curio storage path inside container (long-term)
pub const CURIO_LONG_TERM_STORAGE_PATH: &str = "/home/foc-user/curio/long-term-storage";

pub const CURIO_LAYERS: &str = "pdp-only,gui";

/// PDP layer configuration template
pub const PDP_LAYER_CONFIG_TEMPLATE: &str = r#"[HTTP]
DelegateTLS = true
DomainName = "pdp-sp-{sp_index}.foc-devnet.internal"
Enable = true
ListenAddress = "0.0.0.0:4702"

[Subsystems]
EnableCommP = true
EnableMoveStorage = true
EnablePDP = true
EnableParkPiece = true
"#;

/// Wait times (in seconds)
pub const DB_SETUP_WAIT_SECS: u64 = 10;
pub const STORAGE_ATTACH_WAIT_SECS: u64 = 5;
pub const PDP_KEY_IMPORT_WAIT_SECS: u64 = 5;

/// Verification test file size
pub const TEST_FILE_SIZE_BYTES: usize = 1024; // 1KB
