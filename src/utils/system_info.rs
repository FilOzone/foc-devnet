//! System information utilities.
//!
//! This module provides functions to gather and display system information
//! such as CPU details, core count, and available memory.

use tracing::info;

/// Log system information including CPU, cores, threads, and memory.
pub fn log_system_info() {
    info!("=== System Information ===");
    
    // CPU information
    if let Some(cpu_info) = get_cpu_info() {
        info!("CPU: {}", cpu_info);
    }
    
    // Core count
    let num_cores = num_cpus::get_physical();
    info!("Physical CPU cores: {}", num_cores);
    
    // Thread count
    let num_threads = num_cpus::get();
    info!("Logical CPU threads: {}", num_threads);
    
    // Memory information
    if let Some(total_memory) = get_total_memory() {
        info!("Total RAM: {}", format_bytes(total_memory));
        
        if let Some(available_memory) = get_available_memory() {
            info!("Available RAM: {}", format_bytes(available_memory));
        }
    }
    
    info!("==========================");
}

/// Get CPU model information from /proc/cpuinfo (Linux only).
#[cfg(target_os = "linux")]
fn get_cpu_info() -> Option<String> {
    use std::fs;
    
    let contents = fs::read_to_string("/proc/cpuinfo").ok()?;
    
    for line in contents.lines() {
        if line.starts_with("model name") {
            if let Some(cpu_name) = line.split(':').nth(1) {
                return Some(cpu_name.trim().to_string());
            }
        }
    }
    
    None
}

/// Get CPU model information (macOS).
#[cfg(target_os = "macos")]
fn get_cpu_info() -> Option<String> {
    use std::process::Command;
    
    let output = Command::new("sysctl")
        .arg("-n")
        .arg("machdep.cpu.brand_string")
        .output()
        .ok()?;
    
    String::from_utf8(output.stdout)
        .ok()
        .map(|s| s.trim().to_string())
}

/// Get CPU model information (other platforms).
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn get_cpu_info() -> Option<String> {
    None
}

/// Get total system memory in bytes.
#[cfg(target_os = "linux")]
fn get_total_memory() -> Option<u64> {
    use std::fs;
    
    let contents = fs::read_to_string("/proc/meminfo").ok()?;
    
    for line in contents.lines() {
        if line.starts_with("MemTotal:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let kb = parts[1].parse::<u64>().ok()?;
                return Some(kb * 1024); // Convert KB to bytes
            }
        }
    }
    
    None
}

/// Get available system memory in bytes.
#[cfg(target_os = "linux")]
fn get_available_memory() -> Option<u64> {
    use std::fs;
    
    let contents = fs::read_to_string("/proc/meminfo").ok()?;
    
    for line in contents.lines() {
        if line.starts_with("MemAvailable:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let kb = parts[1].parse::<u64>().ok()?;
                return Some(kb * 1024); // Convert KB to bytes
            }
        }
    }
    
    None
}

/// Get total system memory (macOS).
#[cfg(target_os = "macos")]
fn get_total_memory() -> Option<u64> {
    use std::process::Command;
    
    let output = Command::new("sysctl")
        .arg("-n")
        .arg("hw.memsize")
        .output()
        .ok()?;
    
    String::from_utf8(output.stdout)
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
}

/// Get available system memory (macOS).
#[cfg(target_os = "macos")]
fn get_available_memory() -> Option<u64> {
    // On macOS, we can use vm_stat to get free memory
    // This is an approximation as macOS memory management is complex
    use std::process::Command;
    
    let output = Command::new("vm_stat").output().ok()?;
    let output_str = String::from_utf8(output.stdout).ok()?;
    
    // Parse page size and free pages
    let mut page_size = 4096u64; // Default page size
    let mut free_pages = 0u64;
    
    for line in output_str.lines() {
        if line.contains("page size of") {
            if let Some(size_str) = line.split("page size of").nth(1) {
                if let Some(size) = size_str.split_whitespace().next() {
                    page_size = size.parse().unwrap_or(4096);
                }
            }
        } else if line.starts_with("Pages free:") {
            if let Some(pages) = line.split(':').nth(1) {
                free_pages = pages
                    .trim()
                    .trim_end_matches('.')
                    .parse()
                    .unwrap_or(0);
            }
        }
    }
    
    Some(free_pages * page_size)
}

/// Get total/available memory (other platforms).
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn get_total_memory() -> Option<u64> {
    None
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn get_available_memory() -> Option<u64> {
    None
}

/// Format bytes into a human-readable string.
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    
    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}
