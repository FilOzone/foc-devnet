use crate::paths::foc_devnet_keys;
use bip39::{Language, Mnemonic};
use std::fs;

/// Save mnemonic to file.
/// This is used to persist the generated mnemonic for future key derivation.
/// Default: ~/.foc-devnet/keys/mnemonic.txt
pub fn store_mnemonic(mnemonic: &Mnemonic) -> Result<(), Box<dyn std::error::Error>> {
    let keys_dir = foc_devnet_keys();
    fs::create_dir_all(&keys_dir)?;
    let mnemonic_file = keys_dir.join("mnemonic.txt");
    fs::write(mnemonic_file, mnemonic.to_string())?;
    Ok(())
}

/// Load mnemonic from file.
/// This is used to persist the generated mnemonic for future key derivation.
/// Default: ~/.foc-devnet/keys/mnemonic.txt
pub fn load_mnemonic() -> Result<Mnemonic, Box<dyn std::error::Error>> {
    let keys_dir = foc_devnet_keys();
    let mnemonic_file = keys_dir.join("mnemonic.txt");
    let mnemonic_str = fs::read_to_string(mnemonic_file)?;
    let mnemonic = Mnemonic::parse_in_normalized(Language::English, &mnemonic_str)?;
    Ok(mnemonic)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::TestHome;

    #[test]
    fn store_then_load_mnemonic_round_trips() {
        let home = tempfile::TempDir::new().unwrap();
        let _home = TestHome::new(home.path());

        let original = Mnemonic::from_entropy_in(Language::English, &[42u8; 32]).unwrap();
        store_mnemonic(&original).unwrap();

        let mnemonic_file = foc_devnet_keys().join("mnemonic.txt");
        assert!(mnemonic_file.is_file());

        let loaded = load_mnemonic().unwrap();
        assert_eq!(loaded.to_string(), original.to_string());
    }

    #[test]
    fn load_mnemonic_fails_when_no_mnemonic_was_stored() {
        let home = tempfile::TempDir::new().unwrap();
        let _home = TestHome::new(home.path());

        assert!(load_mnemonic().is_err());
    }
}
