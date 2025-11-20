use crate::paths::{foc_localnet_bin, foc_localnet_curio_repo, foc_localnet_lotus_repo};
use chrono::{DateTime, Utc};
use crossterm::style::Stylize;
use std::process::Command;

/// Execute the status command.
///
/// This function displays a pretty-printed status of the foc-localnet system,
/// including code version, build status, running status, and uptime information.
pub fn status() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "🚀 FOC LocalNet Status".bold().cyan());
    println!("{}", "═".repeat(80).blue());

    // Code version information
    print_code_version()?;

    // Artifacts build status
    print_build_status()?;

    // System running status
    print_running_status()?;

    // Uptime information (if running)
    print_uptime()?;

    Ok(())
}

/// Print code version information in tabular format
fn print_code_version() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📋 {}", "Code Versions".bold());
    println!("{}", "─".repeat(80).blue());

    // Table header
    println!(
        "{: <15} {: <12} {: <28} {: <12}",
        "Component".bold(),
        "Type".bold(),
        "Version".bold(),
        "Commit".bold()
    );
    println!("{}", "─".repeat(80).blue());

    // Get git information for Lotus
    let lotus_repo = foc_localnet_lotus_repo();
    let lotus_git_info = get_git_info(&lotus_repo)?;

    match lotus_git_info {
        GitInfo::Tag(tag) => {
            println!(
                "{: <15} {: <12} {: <28} {: <12}",
                "🪷 Lotus".cyan(),
                "Tag".green(),
                tag.green(),
                ""
            );
        }
        GitInfo::Branch(branch, commit) => {
            println!(
                "{: <15} {: <12} {: <28} {: <12}",
                "🪷 Lotus".cyan(),
                "Branch".green(),
                branch.green(),
                format!("{}...", &commit[..8]).yellow()
            );
        }
        GitInfo::Commit(commit) => {
            println!(
                "{: <15} {: <12} {: <28} {: <12}",
                "🪷 Lotus".cyan(),
                "Commit".yellow(),
                format!("{}...", &commit[..8]).yellow(),
                ""
            );
        }
        GitInfo::None => {
            println!(
                "{: <15} {: <12} {: <28} {: <12}",
                "🪷 Lotus".cyan(),
                "❓ Unknown".red(),
                "Not found".red(),
                ""
            );
        }
    }

    // Get git information for Curio
    let curio_repo = foc_localnet_curio_repo();
    let curio_git_info = get_git_info(&curio_repo)?;

    match curio_git_info {
        GitInfo::Tag(tag) => {
            println!(
                "{: <15} {: <12} {: <28} {: <12}",
                "🤯 Curio".magenta(),
                "Tag".green(),
                tag.green(),
                ""
            );
        }
        GitInfo::Branch(branch, commit) => {
            println!(
                "{: <15} {: <12} {: <28} {: <12}",
                "🤯 Curio".magenta(),
                "Branch".green(),
                branch.green(),
                format!("{}...", &commit[..8]).yellow()
            );
        }
        GitInfo::Commit(commit) => {
            println!(
                "{: <15} {: <12} {: <28} {: <12}",
                "🤯 Curio".magenta(),
                "Commit".yellow(),
                format!("{}...", &commit[..8]).yellow(),
                ""
            );
        }
        GitInfo::None => {
            println!(
                "{: <15} {: <12} {: <28} {: <12}",
                "🤯 Curio".magenta(),
                "❓ Unknown".red(),
                "Not found".red(),
                ""
            );
        }
    }

    Ok(())
}

/// Print build status of artifacts in tabular format
fn print_build_status() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔨 {}", "Build Status".bold());
    println!("{}", "─".repeat(80).blue());

    // Table header
    println!(
        "{: <12} {: <12} {: <40}",
        "Binary".bold(),
        "Status".bold(),
        "Path".bold()
    );
    println!("{}", "─".repeat(80).blue());

    let bin_dir = foc_localnet_bin();

    // Check for expected binaries
    let expected_binaries = vec!["lotus", "lotus-miner", "curio"];

    for binary in expected_binaries {
        let binary_path = bin_dir.join(binary);
        let status = if binary_path.exists() {
            "✅ Built".green()
        } else {
            "❌ Not built".red()
        };
        let location = if binary_path.exists() {
            binary_path.display().to_string().green()
        } else {
            format!("{}/{}", bin_dir.display(), binary).red()
        };

        println!("{: <12} {: <12} {: <40}", binary, status, location);
    }

    Ok(())
}

/// Print running status of the system in tabular format
fn print_running_status() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n⚙️  {}", "System Status".bold());
    println!("{}", "─".repeat(80).blue());

    // Table header
    println!(
        "{: <15} {: <12} {: <20}",
        "Service".bold(),
        "Status".bold(),
        "Container".bold()
    );
    println!("{}", "─".repeat(80).blue());

    // Check for running Docker containers
    let containers = get_running_containers()?;

    let expected_containers = vec![
        ("Lotus Daemon", "foc-lotus"),
        ("Lotus Miner", "foc-lotus-miner"),
        ("Curio", "foc-curio"),
        ("YugabyteDB", "foc-yugabyte"),
    ];

    let mut all_running = true;
    for (service_name, container_name) in &expected_containers {
        let status = if containers.contains(&container_name.to_string()) {
            "🟢 Running".green()
        } else {
            "🔴 Stopped".red()
        };

        if !containers.contains(&container_name.to_string()) {
            all_running = false;
        }

        println!(
            "{: <15} {: <12} {: <20}",
            service_name, status, container_name
        );
    }

    println!("{}", "─".repeat(80).blue());
    if all_running {
        println!(
            "{} {}",
            "🎉".green(),
            "All services are running!".green().bold()
        );
    } else {
        println!(
            "{} {}",
            "⚠️ ".yellow(),
            "Some services are not running.".yellow()
        );
    }

    Ok(())
}

/// Print uptime information if system is running
fn print_uptime() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n⏱️  {}", "System Uptime".bold());
    println!("{}", "─".repeat(80).blue());

    let containers = get_running_containers()?;

    if containers.is_empty() {
        println!("{} {}", "🔴".red(), "System is not running".red());
        return Ok(());
    }

    // Get the oldest container start time as system start time
    if let Some(start_time) = get_system_start_time()? {
        let now = Utc::now();
        let uptime = now.signed_duration_since(start_time);

        let days = uptime.num_days();
        let hours = uptime.num_hours() % 24;
        let minutes = uptime.num_minutes() % 60;
        let seconds = uptime.num_seconds() % 60;

        let uptime_str = if days > 0 {
            format!("{}d {}h {}m {}s", days, hours, minutes, seconds)
        } else if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, seconds)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, seconds)
        } else {
            format!("{}s", seconds)
        };

        println!(
            "{} {}",
            "🕐 System uptime:".green(),
            uptime_str.green().bold()
        );
    } else {
        println!(
            "{} {}",
            "❓".yellow(),
            "Unable to determine uptime".yellow()
        );
    }

    Ok(())
}

/// Get git version information for a specific repository
fn get_git_info(repo_path: &std::path::Path) -> Result<GitInfo, Box<dyn std::error::Error>> {
    // Try to get tag first
    if let Ok(tag_output) = Command::new("git")
        .args([
            "-C",
            repo_path.to_str().unwrap_or("."),
            "describe",
            "--tags",
            "--exact-match",
        ])
        .output()
    {
        if tag_output.status.success() {
            let tag = String::from_utf8_lossy(&tag_output.stdout)
                .trim()
                .to_string();
            return Ok(GitInfo::Tag(tag));
        }
    }

    // Try to get branch and commit
    if let Ok(branch_output) = Command::new("git")
        .args([
            "-C",
            repo_path.to_str().unwrap_or("."),
            "rev-parse",
            "--abbrev-ref",
            "HEAD",
        ])
        .output()
    {
        if branch_output.status.success() {
            let branch = String::from_utf8_lossy(&branch_output.stdout)
                .trim()
                .to_string();

            if let Ok(commit_output) = Command::new("git")
                .args(["-C", repo_path.to_str().unwrap_or("."), "rev-parse", "HEAD"])
                .output()
            {
                if commit_output.status.success() {
                    let commit = String::from_utf8_lossy(&commit_output.stdout)
                        .trim()
                        .to_string();
                    return Ok(GitInfo::Branch(branch, commit));
                }
            }
        }
    }

    // Fallback to just commit hash
    if let Ok(commit_output) = Command::new("git")
        .args(["-C", repo_path.to_str().unwrap_or("."), "rev-parse", "HEAD"])
        .output()
    {
        if commit_output.status.success() {
            let commit = String::from_utf8_lossy(&commit_output.stdout)
                .trim()
                .to_string();
            return Ok(GitInfo::Commit(commit));
        }
    }

    Ok(GitInfo::None)
}

/// Get list of running Docker containers with foc- prefix
fn get_running_containers() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let output = Command::new("docker")
        .args(["ps", "--filter", "name=foc-", "--format", "{{.Names}}"])
        .output()?;

    if !output.status.success() {
        return Ok(vec![]);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let containers: Vec<String> = stdout
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();

    Ok(containers)
}

/// Get the system start time (oldest container start time)
fn get_system_start_time() -> Result<Option<DateTime<Utc>>, Box<dyn std::error::Error>> {
    let output = Command::new("docker")
        .args(["ps", "--filter", "name=foc-", "--format", "{{.RunningFor}}"])
        .output()?;

    if !output.status.success() {
        return Ok(None);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut earliest_start: Option<DateTime<Utc>> = None;

    for line in stdout.lines() {
        if let Some(start_time) = parse_docker_running_for(line.trim()) {
            if earliest_start.is_none() || start_time < earliest_start.unwrap() {
                earliest_start = Some(start_time);
            }
        }
    }

    Ok(earliest_start)
}

/// Parse Docker "Running for" time string into DateTime
fn parse_docker_running_for(running_for: &str) -> Option<DateTime<Utc>> {
    // Docker formats: "2 hours", "3 minutes ago", "About an hour ago", etc.
    // This is a simplified parser - in a real implementation you might want more robust parsing
    let now = Utc::now();

    if running_for.contains("second") {
        let seconds: i64 = running_for.split_whitespace().next()?.parse().ok()?;
        Some(now - chrono::Duration::seconds(seconds))
    } else if running_for.contains("minute") {
        let minutes: i64 = running_for.split_whitespace().next()?.parse().ok()?;
        Some(now - chrono::Duration::minutes(minutes))
    } else if running_for.contains("hour") {
        let hours: i64 = running_for.split_whitespace().next()?.parse().ok()?;
        Some(now - chrono::Duration::hours(hours))
    } else if running_for.contains("day") {
        let days: i64 = running_for.split_whitespace().next()?.parse().ok()?;
        Some(now - chrono::Duration::days(days))
    } else if running_for.contains("week") {
        let weeks: i64 = running_for.split_whitespace().next()?.parse().ok()?;
        Some(now - chrono::Duration::weeks(weeks))
    } else {
        None
    }
}

/// Enum representing different types of git version information
enum GitInfo {
    Tag(String),
    Branch(String, String), // branch name, commit hash
    Commit(String),
    None,
}
