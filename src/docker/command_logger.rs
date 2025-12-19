//! Command logging utilities for tracking executed commands.
//!
//! This module provides utilities to log and track all shell commands executed
//! during the startup process. Commands are saved to the SetupContext for
//! audit, debugging, and troubleshooting purposes.

use crate::commands::start::step::SetupContext;
use std::error::Error;
use std::process::{Command, Output};

/// Format a command and its arguments as a shell string.
///
/// # Arguments
/// * `program` - The program name (e.g., "docker", "git")
/// * `args` - Command arguments as string slices
///
/// # Returns
/// A formatted string representation of the command
///
/// # Example
/// ```
/// let cmd = format_command("docker", &["run", "-it", "ubuntu"]);
/// // Returns: "docker run -it ubuntu"
/// ```
pub fn format_command(program: &str, args: &[&str]) -> String {
    let mut cmd = program.to_string();
    for arg in args {
        // Quote arguments that contain spaces or special characters
        if arg.contains(' ')
            || arg.contains('$')
            || arg.contains('&')
            || arg.contains('>')
            || arg.contains('|')
        {
            cmd.push_str(&format!(" '{}'", arg));
        } else {
            cmd.push_str(&format!(" {}", arg));
        }
    }
    cmd
}

/// Format a command with String arguments.
///
/// # Arguments
/// * `program` - The program name
/// * `args` - Command arguments as Strings
///
/// # Returns
/// A formatted string representation of the command
pub fn format_command_strings(program: &str, args: &[String]) -> String {
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    format_command(program, &arg_refs)
}

/// Execute a command and save it to the context.
///
/// # Arguments
/// * `program` - The program to execute
/// * `args` - Command arguments
/// * `context` - The SetupContext to save the command to
/// * `key` - The key to store the command under in the context
///
/// # Returns
/// The command output
pub fn run_and_log_command(
    program: &str,
    args: &[&str],
    context: &SetupContext,
    key: &str,
) -> Result<Output, Box<dyn Error>> {
    let cmd_string = format_command(program, args);
    context.save_command(key, &cmd_string);
    Command::new(program).args(args).output().map_err(Into::into)
}

/// Execute a command with String arguments and save it to the context.
///
/// # Arguments
/// * `program` - The program to execute
/// * `args` - Command arguments as Strings
/// * `context` - The SetupContext to save the command to
/// * `key` - The key to store the command under in the context
///
/// # Returns
/// The command output
pub fn run_and_log_command_strings(
    program: &str,
    args: &[String],
    context: &SetupContext,
    key: &str,
) -> Result<Output, Box<dyn Error>> {
    let cmd_string = format_command_strings(program, args);
    context.save_command(key, &cmd_string);
    Command::new(program).args(args).output().map_err(Into::into)
}

/// Log a command without executing it (useful for commands already executed).
///
/// # Arguments
/// * `program` - The program name
/// * `args` - Command arguments
/// * `context` - The SetupContext to save the command to
/// * `key` - The key to store the command under in the context
pub fn log_command(program: &str, args: &[&str], context: &SetupContext, key: &str) {
    let cmd_string = format_command(program, args);
    context.save_command(key, &cmd_string);
}

/// Log a command with String arguments without executing it.
pub fn log_command_strings(
    program: &str,
    args: &[String],
    context: &SetupContext,
    key: &str,
) {
    let cmd_string = format_command_strings(program, args);
    context.save_command(key, &cmd_string);
}
