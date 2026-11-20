//! Run ID generation and management.
//!
//! This module handles generating unique run IDs for each cluster start.
//! Format: YYYYMMDDTHHMM-random-name (e.g., 20251203T1246-thirsty-wolf)

mod persistence;

pub use persistence::{delete_current_run_id, load_current_run_id, save_current_run_id};

use chrono::Local;
use rand::seq::IndexedRandom;

/// Adjectives for random name generation.
pub const ADJECTIVES: &[&str] = &[
    "Zany", "Goofy", "Wacky", "Derpy", "Loopy", "Bonky", "Doofy", "Dorky", "Ditzy", "Giddy",
    "Snazzy", "Funky", "Janky", "Dippy", "Noodle", "Goony", "Weeny", "Yucky", "Icky", "Borky",
    "Clown", "Sassy", "Spong", "Bloop", "Tizzy", "Quack", "Smol", "Boing", "Honky", "Wonky",
    "Ploop", "Goobs", "Snorty", "Wobly", "Whiff", "Zoomy", "Fizzy", "Klutz", "Pipsy", "Womp",
];

/// Nouns for random name generation
pub const NOUNS: &[&str] = &[
    "Pip", "Dudu", "Bop", "Moo", "Bean", "Tofu", "Peep", "Mimi", "Lolo", "Coco", "Nini", "Kiki",
    "Fifi", "Bubu", "Dodo", "Toto", "Gigi", "Momo", "Pomp", "Paws", "Buns", "Snip", "Dot", "Wig",
    "Bub", "Tike", "Puff", "Boop", "Zuzu", "Nubs", "Cub", "Toad", "Pig", "Bug", "Mooz", "Pika",
    "Lulu", "Bear", "Fig", "Boo",
];

/// Generate and atomically reserve a unique run ID for this execution.
///
/// The ID uses condensed ISO8601 plus a readable random name. Its run
/// directory is created atomically, so concurrent processes cannot claim the
/// same ID.
pub fn generate_run_id() -> Result<String, Box<dyn std::error::Error>> {
    reserve_run_id_in(&crate::paths::foc_devnet_runs())
}

/// Generate a readable candidate ID without reserving it.
fn generate_candidate() -> String {
    let datetime = Local::now().format("%Y%m%dT%H%M");
    let rng = &mut rand::rng();
    let adjective = ADJECTIVES.choose(rng).unwrap();
    let noun = NOUNS.choose(rng).unwrap();
    format!("{}_{}{}", datetime, adjective, noun)
}

/// Reserve an unused run ID under the supplied runs directory.
fn reserve_run_id_in(runs_dir: &std::path::Path) -> Result<String, Box<dyn std::error::Error>> {
    std::fs::create_dir_all(runs_dir)?;
    loop {
        let run_id = generate_candidate();
        match std::fs::create_dir(runs_dir.join(&run_id)) {
            Ok(()) => return Ok(run_id),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }
}

/// Create a symlink to the latest run directory.
///
/// This function is responsible for maintaining `~/.foc-devnet/state/latest`,
/// a symlink that always points to the most recent run directory.
///
/// It handles:
/// - Creating the parent directory if needed
/// - Removing any existing symlink (including broken ones)
/// - Creating the new symlink
///
/// # Arguments
/// * `run_id` - The run ID of the current execution
///
/// # Failures
/// This is a critical operation. If it fails, the state directory will be
/// inconsistent and subsequent runs may have issues.
pub fn create_latest_symlink(run_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let latest_link = crate::paths::foc_devnet_state_latest();
    let run_dir = crate::paths::foc_devnet_run_dir(run_id);

    // Ensure parent directory exists (state/) before trying to create symlink
    if let Some(parent) = latest_link.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Remove existing symlink if it exists (including broken symlinks)
    // We need to handle broken symlinks carefully:
    // - exists() returns false for broken symlinks (target doesn't exist)
    // - is_symlink() returns true even if target is broken
    // So we check is_symlink() first, which is true for both valid and broken symlinks
    if latest_link.is_symlink() {
        #[cfg(unix)]
        std::fs::remove_file(&latest_link)?;
        #[cfg(windows)]
        std::fs::remove_file(&latest_link)?;
    } else if latest_link.exists() {
        // It's a real directory or file (shouldn't happen, but handle it)
        if latest_link.is_dir() {
            std::fs::remove_dir_all(&latest_link)?;
        } else {
            std::fs::remove_file(&latest_link)?;
        }
    }

    // Create new symlink pointing to the run directory
    #[cfg(unix)]
    std::os::unix::fs::symlink(&run_dir, &latest_link)?;
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(&run_dir, &latest_link)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;

    #[test]
    fn test_run_id_format() {
        let run_id = generate_candidate();

        // Should match pattern: YYYYMMDDTHHMM_RandomName (condensed ISO8601, no dashes/colons)
        let pattern = Regex::new(r"^\d{8}T\d{4}_.+$").unwrap();
        assert!(
            pattern.is_match(&run_id),
            "Run ID should match format: {}",
            run_id
        );
    }

    #[test]
    fn test_reserved_run_ids_are_different() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let id1 = reserve_run_id_in(temp_dir.path()).unwrap();
        let id2 = reserve_run_id_in(temp_dir.path()).unwrap();

        assert_ne!(id1, id2, "Reserved run IDs should be different");
        assert!(temp_dir.path().join(id1).is_dir());
        assert!(temp_dir.path().join(id2).is_dir());
    }

    #[cfg(unix)]
    #[test]
    fn test_create_latest_symlink_replaces_broken_symlink() {
        use crate::paths::{foc_devnet_run_dir, foc_devnet_state_latest, TestHome};
        use std::os::unix::fs::symlink;

        let home = tempfile::TempDir::new().expect("Failed to create temp dir");
        let _home = TestHome::new(home.path());

        let run_dir = foc_devnet_run_dir("run2");
        std::fs::create_dir_all(&run_dir).expect("Failed to create run dir");

        let latest_link = foc_devnet_state_latest();
        std::fs::create_dir_all(latest_link.parent().unwrap()).expect("Failed to create state dir");

        // Point the existing link at a directory that does not exist.
        symlink(home.path().join("run/missing"), &latest_link)
            .expect("Failed to create broken symlink");
        assert!(latest_link.is_symlink());
        assert!(!latest_link.exists(), "broken link target must not exist");

        create_latest_symlink("run2").unwrap();

        assert!(latest_link.is_symlink(), "latest should be recreated");
        assert_eq!(std::fs::read_link(&latest_link).unwrap(), run_dir);
        assert!(latest_link.exists(), "latest target should exist");
    }

    #[cfg(unix)]
    #[test]
    fn test_create_latest_symlink_repoints_existing_link() {
        use crate::paths::{foc_devnet_run_dir, foc_devnet_state_latest, TestHome};

        let home = tempfile::TempDir::new().expect("Failed to create temp dir");
        let _home = TestHome::new(home.path());

        std::fs::create_dir_all(foc_devnet_run_dir("run1")).unwrap();
        std::fs::create_dir_all(foc_devnet_run_dir("run2")).unwrap();

        create_latest_symlink("run1").unwrap();
        assert_eq!(
            std::fs::read_link(foc_devnet_state_latest()).unwrap(),
            foc_devnet_run_dir("run1")
        );

        create_latest_symlink("run2").unwrap();
        assert_eq!(
            std::fs::read_link(foc_devnet_state_latest()).unwrap(),
            foc_devnet_run_dir("run2")
        );
    }
}
