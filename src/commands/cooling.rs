use crate::cli::CoolingCommands;
use log::{debug, info};
use std::fs;
use std::path::Path;
use zbus::Connection;
use zbus::proxy;

#[proxy(
    interface = "org.lapctl1",
    default_service = "org.lapctl",
    default_path = "/org/lapctl"
)]
trait Lapctl {
    async fn set_cooling_profile(&self, profile: String) -> zbus::Result<()>;
}

fn try_call_daemon(command: &CoolingCommands) -> bool {
    if std::env::var("LAPCTL_DAEMON_INTERNAL").is_ok() {
        debug!("Internal daemon call detected. Skipping D-Bus self-call.");
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

        let res = match command {
            CoolingCommands::Performance => {
                tokio::time::timeout(
                    std::time::Duration::from_secs(2),
                    proxy.set_cooling_profile("performance".to_string()),
                )
                .await
            }
            CoolingCommands::Balanced => {
                tokio::time::timeout(
                    std::time::Duration::from_secs(2),
                    proxy.set_cooling_profile("balanced".to_string()),
                )
                .await
            }
            CoolingCommands::Quiet => {
                tokio::time::timeout(
                    std::time::Duration::from_secs(2),
                    proxy.set_cooling_profile("quiet".to_string()),
                )
                .await
            }
        };

        matches!(res, Ok(Ok(_)))
    })
}

fn set_ideapad_fan_mode(mode: &str) -> bool {
    let mut success = false;
    if let Ok(entries) = fs::read_dir("/sys/bus/platform/drivers/ideapad_acpi") {
        for entry in entries.flatten() {
            let path = entry.path().join("fan_mode");
            if path.exists() {
                match fs::write(&path, mode) {
                    Ok(_) => {
                        info!("Set Ideapad fan_mode to {}", mode);
                        success = true;
                    }
                    Err(e) => debug!("Failed to write to {}: {}", path.display(), e),
                }
            }
        }
    }
    success
}

fn set_asus_throttle_policy(mode: &str) -> bool {
    let mut success = false;
    let path = Path::new("/sys/devices/platform/asus-nb-wmi/throttle_thermal_policy");
    if path.exists() {
        match fs::write(path, mode) {
            Ok(_) => {
                info!("Set ASUS WMI thermal policy to {}", mode);
                success = true;
            }
            Err(e) => debug!("Failed to write to {}: {}", path.display(), e),
        }
    }
    success
}

pub fn execute(command: &CoolingCommands) {
    if try_call_daemon(command) {
        println!("Request handled by lapctld daemon.");
        return;
    }

    let (ideapad_mode, asus_mode, desc) = match command {
        CoolingCommands::Performance => ("1", "1", "Performance"),
        CoolingCommands::Balanced => ("0", "0", "Balanced"),
        CoolingCommands::Quiet => ("2", "2", "Quiet"),
    };

    println!("Setting thermal/cooling profile to {}", desc);

    let mut applied = false;
    if set_ideapad_fan_mode(ideapad_mode) {
        applied = true;
        println!("Ideapad fan mode applied.");
    }

    if set_asus_throttle_policy(asus_mode) {
        applied = true;
        println!("ASUS thermal policy applied.");
    }

    if applied {
        println!("Operation completed successfully.");
    } else {
        println!("Hardware does not respond to standard ACPI thermal overrides (Ideapad/ASUS).");
    }
}
