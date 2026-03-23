use log::{error, info};
use std::process::Command;
use zbus::Connection;
use zbus::proxy;

#[proxy(
    interface = "org.lapctl1",
    default_service = "org.lapctl",
    default_path = "/org/lapctl"
)]
trait Lapctl {
    async fn set_system_inhibition(
        &self,
        active: bool,
        why: String,
        who: String,
    ) -> zbus::Result<()>;
}

fn try_call_daemon_inhibit(active: bool, why: String, who: String) -> bool {
    if std::env::var("LAPCTL_DAEMON_INTERNAL").is_ok() {
        return false;
    }

    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return false,
    };

    rt.block_on(async {
        let connection =
            match tokio::time::timeout(std::time::Duration::from_secs(2), Connection::system())
                .await
            {
                Ok(Ok(conn)) => conn,
                _ => return false,
            };

        let proxy = match LapctlProxy::new(&connection).await {
            Ok(p) => p,
            Err(_) => return false,
        };

        let res = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            proxy.set_system_inhibition(active, why, who),
        )
        .await;

        matches!(res, Ok(Ok(_)))
    })
}

pub fn execute(command_args: &[String], why: &str, who: &str, daemon: bool, stop: bool) {
    if stop {
        if try_call_daemon_inhibit(false, why.to_string(), who.to_string()) {
            println!("Persistent system inhibition deactivated via lapctld.");
        } else {
            error!("Failed to deactivate inhibition via daemon. Is lapctld running?");
        }
        return;
    }

    if daemon && command_args.is_empty() {
        if try_call_daemon_inhibit(true, why.to_string(), who.to_string()) {
            println!("Persistent system inhibition activated via lapctld.");
            println!("The system will stay awake until you run: lapctl inhibit --stop");
            return;
        } else {
            error!("Failed to activate inhibition via daemon. Is lapctld running?");
            // Fallback to local systemd-inhibit if daemon call fails
        }
    }

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
