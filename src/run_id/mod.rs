//! Run ID generation and management.
//!
//! This module handles generating unique run IDs for each cluster start.
//! Format: YYYY-MM-DDTHH:MM-random-name (e.g., 2025-12-03T12:46-thirsty-wolf)

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
/// Returns a string like "2025-12-15T22:06_ZanyPip" where:
/// - 2025-12-15 is the date (YYYY-MM-DD, ISO8601 format)
/// - T is the date/time separator (ISO8601)
/// - 22:06 is the time (HH:MM, 24-hour format)
/// - ZanyPip is the random name (adjective + noun)
///
/// # Example
/// ```no_run
/// let run_id = generate_run_id();
/// println!("{}", run_id); // e.g., "2025-12-15T22:06_ZanyPip"
/// ```
pub fn generate_run_id() -> String {
    let now = Local::now();
    let datetime = now.format("%Y-%m-%dT%H:%M");

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
pub fn create_latest_symlink(run_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let latest_link = crate::paths::foc_devnet_state_latest();
    let run_dir = crate::paths::foc_devnet_run_dir(run_id);

    // Remove existing symlink if it exists
    if latest_link.exists() || latest_link.is_symlink() {
        #[cfg(unix)]
        std::fs::remove_file(&latest_link)?;
        #[cfg(windows)]
        if latest_link.is_dir() {
            std::fs::remove_dir(&latest_link)?;
        } else {
            std::fs::remove_file(&latest_link)?;
        }
    }

    // Create new symlink
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

        // Should match pattern: YYYY-MM-DDTHH:MM_RandomName
        let pattern = Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}_.+$").unwrap();
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
}
