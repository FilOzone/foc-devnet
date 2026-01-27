//! Run ID generation and management.
//!
//! This module handles generating unique run IDs for each cluster start.
//! Format: YYYYMMDDTHHMM-random-name (e.g., 20251203T1246-thirsty-wolf)

mod persistence;

pub use persistence::{delete_current_run_id, load_current_run_id, save_current_run_id};

use chrono::Local;
use rand::seq::SliceRandom;

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

/// Generate a unique run ID for this execution.
///
/// Returns a string like "20251215T2206_ZanyPip" where:
/// - 20251215 is the date (YYYYMMDD, condensed ISO8601 format)
/// - T is the date/time separator (ISO8601)
/// - 2206 is the time (HHMM, 24-hour format, no colons for Docker compatibility)
/// - ZanyPip is the random name (adjective + noun)
///
/// Uses condensed ISO8601 format (no dashes or colons) for Docker network name compatibility.
///
/// # Example
/// ```no_run
/// let run_id = generate_run_id();
/// println!("{}", run_id); // e.g., "20251215T2206_ZanyPip"
/// ```
pub fn generate_run_id() -> String {
    let now = Local::now();
    let datetime = now.format("%Y%m%dT%H%M");

    // Implement our own random name generator to control format
    let random_name = {
        let rng = &mut rand::thread_rng();
        let adjective = ADJECTIVES.choose(rng).unwrap();
        let noun = NOUNS.choose(rng).unwrap();
        format!("{}{}", adjective, noun)
    };

    format!("{}_{}", datetime, random_name)
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
        let run_id = generate_run_id();

        // Should match pattern: YYYYMMDDTHHMM_RandomName (condensed ISO8601, no dashes/colons)
        let pattern = Regex::new(r"^\d{8}T\d{4}_.+$").unwrap();
        assert!(
            pattern.is_match(&run_id),
            "Run ID should match format: {}",
            run_id
        );
    }

    #[test]
    fn test_run_ids_are_different() {
        // Generate multiple IDs in quick succession
        // They should be different due to random names (time might be same)
        let id1 = generate_run_id();
        let id2 = generate_run_id();

        // At least the random name part should differ
        assert_ne!(id1, id2, "Run IDs should be different");
    }

    #[test]
    fn test_create_latest_symlink_handles_broken_symlinks() {
        // This test verifies that create_latest_symlink can remove broken symlinks
        use std::os::unix::fs::symlink;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let run_id = generate_run_id();

        // Create mock run directories and state path
        let runs_dir = temp_dir.path().join("runs");
        let state_dir = temp_dir.path().join("state");
        let latest_link = state_dir.join("latest");

        std::fs::create_dir_all(&runs_dir).expect("Failed to create runs dir");
        std::fs::create_dir_all(&state_dir).expect("Failed to create state dir");

        // Create a run directory
        let run_dir = runs_dir.join(&run_id);
        std::fs::create_dir_all(&run_dir).expect("Failed to create run dir");

        // First, create the symlink
        symlink(&run_dir, &latest_link).expect("Failed to create initial symlink");
        assert!(latest_link.is_symlink(), "Initial symlink should exist");

        // Now delete the target to create a broken symlink
        std::fs::remove_dir_all(&run_dir).expect("Failed to remove run dir");
        assert!(
            latest_link.is_symlink(),
            "Broken symlink should still be detected as symlink"
        );
        assert!(
            !latest_link.exists(),
            "Broken symlink target should not exist"
        );

        // Recreate the run directory
        std::fs::create_dir_all(&run_dir).expect("Failed to recreate run dir");

        // Now test that we can remove the broken symlink and create a new one
        if latest_link.is_symlink() {
            std::fs::remove_file(&latest_link).expect("Failed to remove broken symlink");
        }
        assert!(!latest_link.exists(), "Symlink should be removed");

        symlink(&run_dir, &latest_link).expect("Failed to create new symlink");
        assert!(
            latest_link.is_symlink(),
            "New symlink should exist and point to correct target"
        );
    }
}
