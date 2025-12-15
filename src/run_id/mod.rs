//! Run ID generation and management.
//!
//! This module handles generating unique run IDs for each cluster start.
//! Format: YYMMDD-HHMM-random-name (e.g., 251203-1246-thirsty-wolf)

mod persistence;

pub use persistence::{delete_current_run_id, load_current_run_id, save_current_run_id};

use chrono::Local;
use names::{Generator, Name};

/// Adjectives for random name generation.
pub const ADJECTIVES: &[&str] = &[
    "red", "blue", "pink", "gold", "teal", "lime", "navy", "cyan", "gray", "grey", "aqua", "plum",
    "mint", "rose", "jade", "ruby", "tan", "snow", "coal", "rust", "sand", "clay", "mist", "fog",
    "ice", "sky", "sea", "peach", "opal", "lava", "sage", "moss", "slate", "iris", "onyx",
];

/// Nouns for random name generation
pub const NOUNS: &[&str] = &[
    "cat", "dog", "fox", "owl", "bat", "rat", "pig", "cow", "hen", "bee", "ant", "fly", "bug",
    "ape", "eel", "yak", "wolf", "lion", "mole", "vole", "hare", "frog", "toad", "mink", "seal",
    "crab", "fish", "worm", "slug", "deer", "goat", "duck", "swan", "crow", "boar", "lynx", "shad",
    "tern", "pika", "ibis", "gnat", "clam", "lamb", "puma", "orca", "skua", "tuna", "bass", "myna",
    "gull", "newt", "fowl", "dove", "cusk", "cavy", "croc", "paca",
];

/// Generate a unique run ID for this execution.
///
/// Returns a string like "251203-1246-thirsty-wolf" where:
/// - 251203 is the date (YYMMDD)
/// - 1246 is the time (HHMM)
/// - thirsty-wolf is a randomly generated name
///
/// # Example
/// ```no_run
/// let run_id = generate_run_id();
/// println!("{}", run_id); // e.g., "251203-1246-thirsty-wolf"
/// ```
pub fn generate_run_id() -> String {
    let now = Local::now();
    let timestamp = now.format("%y%m%d-%H%M");

    let mut generator = Generator::new(ADJECTIVES, NOUNS, Name::Plain);
    let random_name = generator.next().unwrap_or_else(|| "unknown".to_string());

    format!("{}-{}", timestamp, random_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;

    #[test]
    fn test_run_id_format() {
        let run_id = generate_run_id();

        // Should match pattern: YYMMDD-HHMM-word-word (or word-word-number)
        let pattern = Regex::new(r"^\d{6}-\d{4}-.+$").unwrap();
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
