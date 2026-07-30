//! Docker bind mount preparation.
//!
//! This module ensures bind source directories exist and are writable before
//! they are passed to Docker.

use std::error::Error;
use std::fs;
use std::path::Path;

/// Create a bind source directory and verify the current user can write to it.
pub fn prepare_bind_source(source: &Path) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(source).map_err(|error| {
        format!(
            "Failed to create Docker bind source {}: {}",
            source.display(),
            error
        )
    })?;
    verify_bind_source_writable(source)
}

/// Verify the current user can create files in a bind source directory.
fn verify_bind_source_writable(source: &Path) -> Result<(), Box<dyn Error>> {
    tempfile::tempfile_in(source)
        .map_err(|error| {
            format!(
                "Docker bind source is not writable by the current user: {}: {}",
                source.display(),
                error
            )
            .into()
        })
        .map(|_| ())
}

/// Build a validated directory bind mount value for Docker `--mount`.
pub fn bind_mount(source: &Path, target: &str) -> Result<String, Box<dyn Error>> {
    bind_mount_with_mode(source, target, false)
}

/// Append a validated directory bind mount to Docker command arguments.
pub fn push_bind_mount(
    args: &mut Vec<String>,
    source: &Path,
    target: &str,
) -> Result<(), Box<dyn Error>> {
    push_bind_mount_with_mode(args, source, target, false)
}

/// Append a validated read-only directory bind mount to Docker arguments.
pub fn push_read_only_bind_mount(
    args: &mut Vec<String>,
    source: &Path,
    target: &str,
) -> Result<(), Box<dyn Error>> {
    push_bind_mount_with_mode(args, source, target, true)
}

/// Append a validated directory bind mount with the requested access mode.
fn push_bind_mount_with_mode(
    args: &mut Vec<String>,
    source: &Path,
    target: &str,
    read_only: bool,
) -> Result<(), Box<dyn Error>> {
    args.push("--mount".to_string());
    args.push(bind_mount_with_mode(source, target, read_only)?);
    Ok(())
}

/// Build a validated directory bind mount with the requested access mode.
fn bind_mount_with_mode(
    source: &Path,
    target: &str,
    read_only: bool,
) -> Result<String, Box<dyn Error>> {
    prepare_bind_source(source)?;
    let read_only_option = if read_only { ",readonly" } else { "" };
    Ok(format!(
        "type=bind,source={},target={}{}",
        source.display(),
        target,
        read_only_option
    ))
}

#[cfg(test)]
mod tests {
    use super::{bind_mount, push_read_only_bind_mount};

    /// Missing bind sources are created before the mount is returned.
    #[test]
    fn bind_mount_creates_source_directory() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("nested/source");

        let mount = bind_mount(&source, "/workspace").unwrap();

        assert!(source.is_dir());
        assert_eq!(
            mount,
            format!("type=bind,source={},target=/workspace", source.display())
        );
    }

    /// Read-only mounts use Docker's `readonly` mount option.
    #[test]
    fn read_only_mount_appends_mount_arguments() {
        let root = tempfile::tempdir().unwrap();
        let mut args = vec!["run".to_string()];

        push_read_only_bind_mount(&mut args, root.path(), "/data").unwrap();

        assert_eq!(args[1], "--mount");
        assert_eq!(
            args[2],
            format!(
                "type=bind,source={},target=/data,readonly",
                root.path().display()
            )
        );
    }
}
