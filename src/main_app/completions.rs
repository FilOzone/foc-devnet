//! Shell completion generation and installation.
//!
//! This module handles generating and installing shell completion scripts.

use clap::CommandFactory;
use clap_complete::{generate, Shell};
use foc_localnet::cli::Cli;

/// Execute the completions command
pub fn handle_completions(
    shell: Option<String>,
    install: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Completions generation is read-only, no poison protection needed
    let shell = match shell {
        Some(shell_name) => {
            match shell_name.as_str() {
                "bash" => Shell::Bash,
                "zsh" => Shell::Zsh,
                "fish" => Shell::Fish,
                "powershell" => Shell::PowerShell,
                "elvish" => Shell::Elvish,
                _ => {
                    eprintln!("Unsupported shell: {}. Supported shells: bash, zsh, fish, powershell, elvish", shell_name);
                    std::process::exit(1);
                }
            }
        }
        None => {
            // Try to detect shell from $SHELL environment variable
            match std::env::var("SHELL").ok().and_then(|shell_path| {
                if shell_path.contains("bash") {
                    Some(Shell::Bash)
                } else if shell_path.contains("zsh") {
                    Some(Shell::Zsh)
                } else if shell_path.contains("fish") {
                    Some(Shell::Fish)
                } else {
                    None
                }
            }) {
                Some(detected_shell) => detected_shell,
                None => {
                    eprintln!("Could not detect shell from $SHELL environment variable.");
                    eprintln!(
                        "Please specify the shell explicitly: foc-localnet completions <shell>"
                    );
                    eprintln!("Supported shells: bash, zsh, fish, powershell, elvish");
                    std::process::exit(1);
                }
            }
        }
    };

    if install {
        install_completions(shell)?;
    } else {
        // Just output to stdout
        generate(
            shell,
            &mut Cli::command(),
            "foc-localnet",
            &mut std::io::stdout(),
        );
    }

    Ok(())
}

/// Install completion script to the appropriate location for the shell
fn install_completions(shell: Shell) -> Result<(), Box<dyn std::error::Error>> {
    let (completion_path, completion_dir) = match shell {
        Shell::Bash => get_bash_completion_paths(),
        Shell::Zsh => get_zsh_completion_paths(),
        Shell::Fish => get_fish_completion_paths(),
        Shell::PowerShell => get_powershell_completion_paths(),
        Shell::Elvish => get_elvish_completion_paths(),
        _ => {
            eprintln!(
                "Installation not supported for this shell. Please generate the script manually:"
            );
            eprintln!("  foc-localnet completions {} > completion_script", shell);
            std::process::exit(1);
        }
    };

    // Create directory if it doesn't exist
    if let Err(e) = std::fs::create_dir_all(&completion_dir) {
        eprintln!(
            "Failed to create completion directory {}: {}",
            completion_dir, e
        );
        std::process::exit(1);
    }

    // Generate completion script to file
    let mut file = match std::fs::File::create(&completion_path) {
        Ok(file) => file,
        Err(e) => {
            eprintln!(
                "Failed to create completion file {}: {}",
                completion_path, e
            );
            std::process::exit(1);
        }
    };

    // Generate completion script to file
    clap_complete::generate(shell, &mut Cli::command(), "foc-localnet", &mut file);

    println!("✅ Completion script installed to: {}", completion_path);
    println!("💡 You may need to restart your shell or source the completion file for changes to take effect.");

    Ok(())
}

/// Get completion paths for Bash shell
fn get_bash_completion_paths() -> (String, String) {
    // Try system location first if writable, otherwise user location
    let system_dir = "/etc/bash_completion.d";
    let system_path = "/etc/bash_completion.d/foc-localnet";
    if std::path::Path::new(system_dir).exists() {
        // Check if we can write to the system location
        if let Ok(metadata) = std::fs::metadata(system_dir) {
            if !metadata.permissions().readonly() {
                // Try to create a test file to check write permission
                let test_file = format!("{}/.foc-test", system_dir);
                if std::fs::File::create(&test_file).is_ok() {
                    let _ = std::fs::remove_file(&test_file); // Clean up test file
                    return (system_path.to_string(), system_dir.to_string());
                }
            }
        }
    }

    // Fall back to user location
    let user_dir = format!("{}/.bash_completion.d", dirs::home_dir().unwrap().display());
    let user_path = format!(
        "{}/.bash_completion.d/foc-localnet",
        dirs::home_dir().unwrap().display()
    );
    (user_path, user_dir)
}

/// Get completion paths for Zsh shell
fn get_zsh_completion_paths() -> (String, String) {
    // Try system location first if writable, otherwise user location
    let system_dir = "/usr/local/share/zsh/site-functions";
    let system_path = "/usr/local/share/zsh/site-functions/_foc-localnet";
    if std::path::Path::new(system_dir).exists() {
        // Check if we can write to the system location
        if let Ok(metadata) = std::fs::metadata(system_dir) {
            if !metadata.permissions().readonly() {
                // Try to create a test file to check write permission
                let test_file = format!("{}/.foc-test", system_dir);
                if std::fs::File::create(&test_file).is_ok() {
                    let _ = std::fs::remove_file(&test_file); // Clean up test file
                    return (system_path.to_string(), system_dir.to_string());
                }
            }
        }
    }

    // Fall back to user location
    let user_path = format!(
        "{}/.zsh/completions/_foc-localnet",
        dirs::home_dir().unwrap().display()
    );
    let user_dir = format!("{}/.zsh/completions", dirs::home_dir().unwrap().display());
    (user_path, user_dir)
}

/// Get completion paths for Fish shell
fn get_fish_completion_paths() -> (String, String) {
    let user_path = format!(
        "{}/.config/fish/completions/foc-localnet.fish",
        dirs::home_dir().unwrap().display()
    );
    let user_dir = format!(
        "{}/.config/fish/completions",
        dirs::home_dir().unwrap().display()
    );
    (user_path, user_dir)
}

/// Get completion paths for PowerShell
fn get_powershell_completion_paths() -> (String, String) {
    let user_path = format!(
        "{}/.config/powershell/foc-localnet.ps1",
        dirs::home_dir().unwrap().display()
    );
    let user_dir = format!("{}/.config/powershell", dirs::home_dir().unwrap().display());
    (user_path, user_dir)
}

/// Get completion paths for Elvish shell
fn get_elvish_completion_paths() -> (String, String) {
    let user_path = format!(
        "{}/.config/elvish/rc.elv",
        dirs::home_dir().unwrap().display()
    );
    let user_dir = format!("{}/.config/elvish", dirs::home_dir().unwrap().display());
    (user_path, user_dir)
}
