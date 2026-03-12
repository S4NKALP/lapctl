use log::{error, info};
use std::process::Command;

pub fn execute(command_args: &[String], why: &str, who: &str, daemon: bool) {
    let mut args = vec![
        "--why".to_string(),
        why.to_string(),
        "--who".to_string(),
        who.to_string(),
        "--what".to_string(),
        "sleep:idle".to_string(),
    ];

    if command_args.is_empty() {
        if !daemon {
            info!("No command provided. Inhibiting sleep indefinitely. Press Ctrl+C to stop.");
        }
        args.push("sleep".to_string());
        args.push("infinity".to_string());
    } else {
        info!(
            "Running command while inhibiting sleep: {}",
            command_args.join(" ")
        );
        args.extend(command_args.iter().cloned());
    }

    if daemon {
        match Command::new("systemd-inhibit").args(&args).spawn() {
            Ok(child) => {
                info!("Inhibitor started in background with PID: {}", child.id());
                info!("To stop it, use: pkill -P {} systemd-inhibit", child.id());
                // In background mode, we just let the child run and exit lapctl
            }
            Err(e) => {
                error!("Failed to spawn systemd-inhibit: {}", e);
            }
        }
    } else {
        match Command::new("systemd-inhibit").args(&args).status() {
            Ok(status) => {
                if !status.success() {
                    error!("systemd-inhibit exited with status: {}", status);
                }
            }
            Err(e) => {
                error!("Failed to execute systemd-inhibit: {}", e);
                eprintln!(
                    "Error: make sure 'systemd-inhibit' is installed and available in your PATH."
                );
            }
        }
    }
}
