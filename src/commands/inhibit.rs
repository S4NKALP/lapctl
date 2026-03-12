use log::{error, info};
use std::process::Command;

pub fn execute(command_args: &[String], why: &str, who: &str, daemon: bool) {
    let args = build_args(command_args, why, who);

    if command_args.is_empty() {
        if !daemon {
            info!("No command provided. Inhibiting sleep indefinitely. Press Ctrl+C to stop.");
        }
    } else {
        info!(
            "Running command while inhibiting sleep: {}",
            command_args.join(" ")
        );
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

pub(crate) fn build_args(command_args: &[String], why: &str, who: &str) -> Vec<String> {
    let mut args = vec![
        "--why".to_string(),
        why.to_string(),
        "--who".to_string(),
        who.to_string(),
        "--what".to_string(),
        "sleep:idle".to_string(),
    ];

    if command_args.is_empty() {
        args.push("sleep".to_string());
        args.push("infinity".to_string());
    } else {
        args.extend(command_args.iter().cloned());
    }
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_args_no_command() {
        let args = build_args(&[], "testing", "lapctl");
        assert_eq!(
            args,
            vec![
                "--why",
                "testing",
                "--who",
                "lapctl",
                "--what",
                "sleep:idle",
                "sleep",
                "infinity"
            ]
        );
    }

    #[test]
    fn test_build_args_with_command() {
        let command_args = vec!["ls".to_string(), "-l".to_string()];
        let args = build_args(&command_args, "testing", "lapctl");
        assert_eq!(
            args,
            vec![
                "--why",
                "testing",
                "--who",
                "lapctl",
                "--what",
                "sleep:idle",
                "ls",
                "-l"
            ]
        );
    }
}
