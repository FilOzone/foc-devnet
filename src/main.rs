use clap::{CommandFactory, Parser};
use clap_complete::{generate, Shell};
use crossterm::style::Stylize;
use foc_localnet::cli::{BuildCommands, Cli, Commands, ConfigCommands};
use foc_localnet::commands;
use foc_localnet::commands::build::Project;
use foc_localnet::config::Config;
use foc_localnet::paths::foc_localnet_config;
use foc_localnet::poison;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Check for poison file and attempt recovery
    poison::check_and_recover_poison()?;

    // Execute the command with poison file protection
    let result = match cli.command {
        Commands::Start {
            volumes_dir,
            logs_dir,
            regenesis,
            reset,
        } => {
            poison::create_poison("Start")?;
            commands::start_cluster(volumes_dir, logs_dir, regenesis, reset)
        }
        Commands::Stop => {
            poison::create_poison("Stop")?;
            commands::stop_cluster()
        }
        Commands::Requirements { setup } => {
            poison::create_poison("Requirements")?;
            commands::check_requirements(setup)
        }
        Commands::Init {
            curio,
            lotus,
            filecoin_services,
            yugabyte_url,
            force,
            rand,
        } => {
            poison::create_poison("Init")?;
            commands::init_environment(curio, lotus, filecoin_services, yugabyte_url, force, rand)
        }
        Commands::Build { build_command } => {
            poison::create_poison("Build")?;

            // Load configuration
            let config_path = foc_localnet_config();
            let config_content = fs::read_to_string(&config_path)
                .map_err(|e| format!("Failed to read config file at {:?}: {}", config_path, e))?;
            let config: Config = toml::from_str(&config_content)
                .map_err(|e| format!("Failed to parse config file: {}", e))?;

            match build_command {
                BuildCommands::Lotus {
                    path: _,
                    output_dir: _,
                } => commands::build_project(&Project::Lotus, &config),
                BuildCommands::Curio {
                    path: _,
                    output_dir: _,
                } => commands::build_project(&Project::Curio, &config),
            }
        }
        Commands::Clean {
            artifacts,
            dockerimages,
            binaries,
            lotus,
            curio,
        } => {
            poison::create_poison("Clean")?;
            commands::clean_environment(artifacts, dockerimages, binaries, lotus, curio, false)
        }
        Commands::Status => {
            // Status is read-only, no poison protection needed
            commands::status()
        }
        Commands::Completions { shell, install } => {
            // Completions generation is read-only, no poison protection needed
            let shell = match shell {
                Some(shell_name) => match shell_name.as_str() {
                    "bash" => Shell::Bash,
                    "zsh" => Shell::Zsh,
                    "fish" => Shell::Fish,
                    "powershell" => Shell::PowerShell,
                    "elvish" => Shell::Elvish,
                    _ => {
                        eprintln!("Unsupported shell: {}. Supported shells: bash, zsh, fish, powershell, elvish", shell_name);
                        std::process::exit(1);
                    }
                },
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
                            eprintln!("Please specify the shell explicitly: foc-localnet completions <shell>");
                            eprintln!("Supported shells: bash, zsh, fish, powershell, elvish");
                            std::process::exit(1);
                        }
                    }
                }
            };

            if install {
                // Install to appropriate location
                let (completion_path, completion_dir) = match shell {
                    Shell::Bash => {
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
                                        (system_path.to_string(), system_dir.to_string())
                                    } else {
                                        // Fall back to user location
                                        let user_dir = format!(
                                            "{}/.bash_completion.d",
                                            dirs::home_dir().unwrap().display()
                                        );
                                        let user_path = format!(
                                            "{}/.bash_completion.d/foc-localnet",
                                            dirs::home_dir().unwrap().display()
                                        );
                                        (user_path, user_dir)
                                    }
                                } else {
                                    // Fall back to user location
                                    let user_dir = format!(
                                        "{}/.bash_completion.d",
                                        dirs::home_dir().unwrap().display()
                                    );
                                    let user_path = format!(
                                        "{}/.bash_completion.d/foc-localnet",
                                        dirs::home_dir().unwrap().display()
                                    );
                                    (user_path, user_dir)
                                }
                            } else {
                                // Fall back to user location
                                let user_dir = format!(
                                    "{}/.bash_completion.d",
                                    dirs::home_dir().unwrap().display()
                                );
                                let user_path = format!(
                                    "{}/.bash_completion.d/foc-localnet",
                                    dirs::home_dir().unwrap().display()
                                );
                                (user_path, user_dir)
                            }
                        } else {
                            let user_dir = format!(
                                "{}/.bash_completion.d",
                                dirs::home_dir().unwrap().display()
                            );
                            let user_path = format!(
                                "{}/.bash_completion.d/foc-localnet",
                                dirs::home_dir().unwrap().display()
                            );
                            (user_path, user_dir)
                        }
                    }
                    Shell::Zsh => {
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
                                        (system_path.to_string(), system_dir.to_string())
                                    } else {
                                        // Fall back to user location
                                        let user_path = format!(
                                            "{}/.zsh/completions/_foc-localnet",
                                            dirs::home_dir().unwrap().display()
                                        );
                                        (
                                            user_path,
                                            format!(
                                                "{}/.zsh/completions",
                                                dirs::home_dir().unwrap().display()
                                            ),
                                        )
                                    }
                                } else {
                                    // Fall back to user location
                                    let user_path = format!(
                                        "{}/.zsh/completions/_foc-localnet",
                                        dirs::home_dir().unwrap().display()
                                    );
                                    (
                                        user_path,
                                        format!(
                                            "{}/.zsh/completions",
                                            dirs::home_dir().unwrap().display()
                                        ),
                                    )
                                }
                            } else {
                                // Fall back to user location
                                let user_path = format!(
                                    "{}/.zsh/completions/_foc-localnet",
                                    dirs::home_dir().unwrap().display()
                                );
                                (
                                    user_path,
                                    format!(
                                        "{}/.zsh/completions",
                                        dirs::home_dir().unwrap().display()
                                    ),
                                )
                            }
                        } else {
                            let user_path = format!(
                                "{}/.zsh/completions/_foc-localnet",
                                dirs::home_dir().unwrap().display()
                            );
                            (
                                user_path,
                                format!("{}/.zsh/completions", dirs::home_dir().unwrap().display()),
                            )
                        }
                    }
                    Shell::Fish => {
                        let user_path = format!(
                            "{}/.config/fish/completions/foc-localnet.fish",
                            dirs::home_dir().unwrap().display()
                        );
                        (
                            user_path,
                            format!(
                                "{}/.config/fish/completions",
                                dirs::home_dir().unwrap().display()
                            ),
                        )
                    }
                    Shell::PowerShell => {
                        let user_path = format!(
                            "{}/.config/powershell/foc-localnet.ps1",
                            dirs::home_dir().unwrap().display()
                        );
                        (
                            user_path,
                            format!("{}/.config/powershell", dirs::home_dir().unwrap().display()),
                        )
                    }
                    Shell::Elvish => {
                        let user_path = format!(
                            "{}/.config/elvish/rc.elv",
                            dirs::home_dir().unwrap().display()
                        );
                        (
                            user_path,
                            format!("{}/.config/elvish", dirs::home_dir().unwrap().display()),
                        )
                    }
                    _ => {
                        eprintln!("Installation not supported for this shell. Please generate the script manually:");
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
        Commands::Version => {
            // Version information is read-only, no poison protection needed
            println!("foc-localnet {}", env!("CARGO_PKG_VERSION"));
            println!("Commit: {}", env!("GIT_COMMIT"));
            println!("Branch: {}", env!("GIT_BRANCH"));

            // Calculate relative time
            let build_timestamp: i64 = env!("BUILD_TIMESTAMP").parse().unwrap_or(0);
            let now = chrono::Utc::now().timestamp();
            let diff_seconds = now - build_timestamp;

            let relative_time = if diff_seconds < 60 {
                format!("({} seconds ago)", diff_seconds)
            } else if diff_seconds < 3600 {
                format!("({} minutes ago)", diff_seconds / 60)
            } else if diff_seconds < 86400 {
                format!("({} hours ago)", diff_seconds / 3600)
            } else {
                format!("({} days ago)", diff_seconds / 86400)
            };

            println!("Built (UTC): {} {}", env!("BUILD_TIME_UTC"), relative_time);
            println!("Built (Local): {}", env!("BUILD_TIME_LOCAL"));
            Ok(())
        }
        Commands::Config { config_command } => {
            poison::create_poison("Config")?;
            match config_command {
                ConfigCommands::Lotus { source } => commands::config_lotus(source),
                ConfigCommands::Curio { source } => commands::config_curio(source),
            }
        }
    };

    // Handle the result
    match result {
        Ok(_) => {
            // Remove poison file on successful completion
            poison::remove_poison()?;
            Ok(())
        }
        Err(e) => {
            // Leave poison file in place on error
            eprintln!(
                "{}",
                "Command failed, poison file left in place for safety".red()
            );
            Err(e)
        }
    }
}
