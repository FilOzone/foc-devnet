use std::fs;
use std::path::Path;
use tracing::info;

/// Initialize the application environment.
///
/// This function sets up the necessary directories and configuration files
/// for the foc-localnet application. It ensures the application directory
/// exists and installs a default configuration if none is present.
pub fn initialize_app() -> Result<(), Box<dyn std::error::Error>> {
    // Check and create application directory
    let app_dir = Path::new("/opt/foc-localnet");
    if !app_dir.exists() {
        info!("Creating application directory: {:?}", app_dir);
        fs::create_dir_all(app_dir)?;
    }

    // Check and install default config
    let config_path = app_dir.join("config.toml");
    if !config_path.exists() {
        info!("Installing default config: {:?}", config_path);
        let default_config = toml::to_string(&crate::config::Config::default()).unwrap();
        fs::write(&config_path, default_config)?;
    }

    Ok(())
}

/// Initialize tracing/logging for the application.
pub fn init_tracing() {
    tracing_subscriber::fmt::init();
}
