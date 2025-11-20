use crate::config::Config;
use crate::paths::{foc_localnet_bin, foc_localnet_code, foc_localnet_config};
use chrono::{DateTime, Utc};
use crossterm::style::Stylize;
use std::fs;
use std::process::Command;
use tabular::{Row, Table};

/// Execute the status command.
///
/// This function displays a pretty-printed status of the foc-localnet system,
/// including code version, build status, running status, and uptime information.
pub fn status() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{}", "🚀 FOC LocalNet Status".bold().cyan().underlined());
    println!("{}", "═".repeat(80).cyan());

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
    println!("\n{} {}", "📋".cyan(), "Code Versions".bold().cyan());
    println!("{}", "─".repeat(80).cyan());

    // Load configuration
    let config_path = foc_localnet_config();
    let config_content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file at {:?}: {}", config_path, e))?;
    let config: Config = toml::from_str(&config_content)
        .map_err(|e| format!("Failed to parse config file: {}", e))?;

    // Get git information for Lotus
    let lotus_repo_path = get_repo_path_from_config(&config.lotus, "lotus");
    let lotus_git_info = get_git_info(&lotus_repo_path)?;

    let (lotus_source_type, lotus_version, lotus_commit, lotus_status) =
        format_location_info(&config.lotus, &lotus_git_info, &lotus_repo_path);

    // Get git information for Curio
    let curio_repo_path = get_repo_path_from_config(&config.curio, "curio");
    let curio_git_info = get_git_info(&curio_repo_path)?;

    let (curio_source_type, curio_version, curio_commit, curio_status) =
        format_location_info(&config.curio, &curio_git_info, &curio_repo_path);

    // Create tabular output with proper column widths
    let mut table = Table::new("{:<}  {:<}  {:<}  {:<}  {:<}  {:<}");
    table.add_row(
        Row::new()
            .with_ansi_cell("Component".bold().dark_grey())
            .with_ansi_cell("Source Type".bold().dark_grey())
            .with_ansi_cell("Branch/Tag".bold().dark_grey())
            .with_ansi_cell("Commit".bold().dark_grey())
            .with_ansi_cell("Status".bold().dark_grey())
            .with_ansi_cell("Code Path".bold().dark_grey()),
    );

    // Use with_ansi_cell for colored output
    table.add_row(
        Row::new()
            .with_ansi_cell("Lotus".cyan())
            .with_ansi_cell(&lotus_source_type)
            .with_ansi_cell(&lotus_version)
            .with_ansi_cell(&lotus_commit)
            .with_ansi_cell(&lotus_status)
            .with_ansi_cell(lotus_repo_path.display().to_string().dim()),
    );

    table.add_row(
        Row::new()
            .with_ansi_cell("Curio".magenta())
            .with_ansi_cell(&curio_source_type)
            .with_ansi_cell(&curio_version)
            .with_ansi_cell(&curio_commit)
            .with_ansi_cell(&curio_status)
            .with_ansi_cell(curio_repo_path.display().to_string().dim()),
    );

    print!("{}", table);

    Ok(())
}

/// Print build status of artifacts in tabular format
fn print_build_status() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{} {}", "🔨".yellow(), "Build Status".bold().yellow());
    println!("{}", "─".repeat(80).yellow());

    let bin_dir = foc_localnet_bin();

    // Check for expected binaries
    let expected_binaries = vec!["lotus", "lotus-miner", "curio"];

    // Create tabular output
    let mut table = Table::new("{:<}  {:<}  {:<}");
    table.add_row(
        Row::new()
            .with_ansi_cell("Binary".bold().dark_grey())
            .with_ansi_cell("Status".bold().dark_grey())
            .with_ansi_cell("Path".bold().dark_grey()),
    );

    for binary in expected_binaries {
        let binary_path = bin_dir.join(binary);
        let status = if binary_path.exists() {
            "Built".green().to_string()
        } else {
            "Not built".red().to_string()
        };
        let location = if binary_path.exists() {
            binary_path.display().to_string()
        } else {
            format!("{}/{}", bin_dir.display(), binary)
        };

        table.add_row(
            Row::new()
                .with_cell(binary)
                .with_ansi_cell(&status)
                .with_ansi_cell(location.dim()),
        );
    }

    print!("{}", table);

    Ok(())
}

/// Print running status of the system in tabular format
fn print_running_status() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{} {}", "⚙️".green(), "System Status".bold().green());
    println!("{}", "─".repeat(80).green());

    // Check for running Docker containers
    let containers = get_running_containers()?;

    let expected_containers = vec![
        ("Lotus Daemon", "foc-lotus"),
        ("Lotus Miner", "foc-lotus-miner"),
        ("Curio", "foc-curio"),
        ("YugabyteDB", "foc-yugabyte"),
    ];

    // Create tabular output
    let mut table = Table::new("{:<}  {:<}  {:<}");
    table.add_row(
        Row::new()
            .with_ansi_cell("Service".bold().dark_grey())
            .with_ansi_cell("Status".bold().dark_grey())
            .with_ansi_cell("Container".bold().dark_grey()),
    );

    let mut all_running = true;
    for (service_name, container_name) in &expected_containers {
        let status = if containers.contains(&container_name.to_string()) {
            "Running".green().to_string()
        } else {
            "Stopped".red().to_string()
        };

        if !containers.contains(&container_name.to_string()) {
            all_running = false;
        }

        table.add_row(
            Row::new()
                .with_cell(*service_name)
                .with_ansi_cell(&status)
                .with_cell(*container_name),
        );
    }

    print!("{}", table);
    println!("{}", "─".repeat(80).green());

    if all_running {
        println!("{}", "All services are running!".green().bold());
    } else {
        println!("{}", "Some services are not running.".yellow());
    }

    Ok(())
}

/// Print uptime information if system is running
fn print_uptime() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{} {}", "⏱️".magenta(), "System Uptime".bold().magenta());
    println!("{}", "─".repeat(80).magenta());

    let containers = get_running_containers()?;

    if containers.is_empty() {
        println!("{}", "System is not running".red());
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

        println!("{} {}", "System uptime:".green(), uptime_str.green().bold());
    } else {
        println!("{}", "Unable to determine uptime".yellow());
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

/// Get the repository path to check for git information based on the config location
fn get_repo_path_from_config(
    location: &crate::config::Location,
    component: &str,
) -> std::path::PathBuf {
    use crate::config::Location;

    match location {
        Location::LocalSource { dir } => {
            // For local sources, check the specified directory
            std::path::PathBuf::from(dir)
        }
        Location::GitTag { .. } | Location::GitCommit { .. } | Location::GitBranch { .. } => {
            // For git sources, check if it exists in the foc-localnet code directory
            foc_localnet_code().join(component)
        }
    }
}

/// Format location and git information for display
fn format_location_info(
    location: &crate::config::Location,
    git_info: &GitInfo,
    _repo_path: &std::path::Path,
) -> (String, String, String, String) {
    use crate::config::Location;

    let is_ready = match (location, git_info) {
        // LocalSource is ready if it has any git info
        (
            Location::LocalSource { .. },
            GitInfo::Tag(_) | GitInfo::Branch(_, _) | GitInfo::Commit(_),
        ) => true,
        (Location::LocalSource { .. }, GitInfo::None) => false,

        // GitTag is ready if the repository has that exact tag checked out
        (
            Location::GitTag {
                tag: expected_tag, ..
            },
            GitInfo::Tag(actual_tag),
        ) if expected_tag == actual_tag => true,
        (Location::GitTag { .. }, _) => false,

        // GitCommit is ready if the repository is at that exact commit
        (
            Location::GitCommit {
                commit: expected_commit,
                ..
            },
            GitInfo::Commit(actual_commit),
        ) if expected_commit == actual_commit => true,
        (Location::GitCommit { .. }, _) => false,

        // GitBranch is ready if the repository is on that branch (or has that branch's commit/tag)
        (
            Location::GitBranch {
                branch: expected_branch,
                ..
            },
            GitInfo::Branch(actual_branch, _),
        ) if expected_branch == actual_branch => true,
        (Location::GitBranch { .. }, GitInfo::Tag(_) | GitInfo::Commit(_)) => true, // Assume it's ready if we have some valid state
        (Location::GitBranch { .. }, _) => false,
    };

    let status = if is_ready {
        "Ready".green().to_string()
    } else {
        "Not Ready".red().to_string()
    };

    let (source_type, version, commit) = match location {
        Location::LocalSource { dir: _ } => match git_info {
            GitInfo::Tag(tag) => ("Local (Git Tag)".to_string(), tag.clone(), "".to_string()),
            GitInfo::Branch(branch, commit) => (
                "Local (Git Branch)".to_string(),
                branch.clone(),
                format!("{}...", &commit[..8]),
            ),
            GitInfo::Commit(commit) => (
                "Local (Git Commit)".to_string(),
                format!("{}...", &commit[..8]),
                "".to_string(),
            ),
            GitInfo::None => ("Local".to_string(), "Not found".to_string(), "".to_string()),
        },
        Location::GitTag { tag, .. } => ("Git Tag".to_string(), tag.clone(), "".to_string()),
        Location::GitCommit { commit, .. } => (
            "Git Commit".to_string(),
            format!("{}...", &commit[..8]),
            "".to_string(),
        ),
        Location::GitBranch { branch, .. } => match git_info {
            GitInfo::Branch(git_branch, commit) => (
                "Git Branch".to_string(),
                git_branch.clone(),
                format!("{}...", &commit[..8]),
            ),
            GitInfo::Tag(tag) => ("Git Branch + Tag".to_string(), tag.clone(), "".to_string()),
            GitInfo::Commit(commit) => (
                "Git Branch + Commit".to_string(),
                format!("{}...", &commit[..8]),
                "".to_string(),
            ),
            GitInfo::None => (
                "Git Branch".to_string(),
                branch.clone(),
                "Not found".to_string(),
            ),
        },
    };

    (source_type, version, commit, status)
}

/// Enum representing different types of git version information
enum GitInfo {
    Tag(String),
    Branch(String, String), // branch name, commit hash
    Commit(String),
    None,
}
