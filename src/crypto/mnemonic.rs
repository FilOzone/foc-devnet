use crate::paths::foc_localnet_keys;
use bip39::{Language, Mnemonic};
use std::fs;

/// Save mnemonic to file.
/// This is used to persist the generated mnemonic for future key derivation.
/// Default: ~/.foc-localnet/keys/mnemonic.txt
pub fn store_mnemonic(mnemonic: &Mnemonic) -> Result<(), Box<dyn std::error::Error>> {
    let keys_dir = foc_localnet_keys();
    fs::create_dir_all(&keys_dir)?;
    let mnemonic_file = keys_dir.join("mnemonic.txt");
    fs::write(mnemonic_file, mnemonic.to_string())?;
    Ok(())
}

/// Load mnemonic from file.
/// This is used to persist the generated mnemonic for future key derivation.
/// Default: ~/.foc-localnet/keys/mnemonic.txt
pub fn load_mnemonic() -> Result<Mnemonic, Box<dyn std::error::Error>> {
    let keys_dir = foc_localnet_keys();
    let mnemonic_file = keys_dir.join("mnemonic.txt");
    let mnemonic_str = fs::read_to_string(mnemonic_file)?;
    let mnemonic = Mnemonic::parse_in_normalized(Language::English, &mnemonic_str)?;
    Ok(mnemonic)
}
