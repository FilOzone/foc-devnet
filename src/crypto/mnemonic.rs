use crate::paths::foc_localnet_state;
use bip39::{Language, Mnemonic};
use std::fs;

/// Save mnemonic to file.
/// This is used to persist the generated mnemonic for future key derivation.
/// Default: ~/.foc-localnet/state/mnemonic.txt
pub fn store_mnemonic(mnemonic: &Mnemonic) -> Result<(), Box<dyn std::error::Error>> {
    let state_dir = foc_localnet_state();
    fs::create_dir_all(&state_dir)?;
    let mnemonic_file = state_dir.join("mnemonic.txt");
    fs::write(mnemonic_file, mnemonic.to_string())?;
    Ok(())
}

/// Load mnemonic from file.
/// This is used to persist the generated mnemonic for future key derivation.
/// Default: ~/.foc-localnet/state/mnemonic.txt
pub fn load_mnemonic() -> Result<Mnemonic, Box<dyn std::error::Error>> {
    let state_dir = foc_localnet_state();
    let mnemonic_file = state_dir.join("mnemonic.txt");
    let mnemonic_str = fs::read_to_string(mnemonic_file)?;
    let mnemonic = Mnemonic::parse_in_normalized(Language::English, &mnemonic_str)?;
    Ok(mnemonic)
}
