use crate::cli::BatteryCommands;
use log::{debug, error, info};
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
    async fn set_battery_limit(&self, percent: u32) -> zbus::Result<()>;
}

fn try_call_daemon(command: &BatteryCommands) -> bool {
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

        let res = match command {
            BatteryCommands::Limit { percent } => {
                tokio::time::timeout(
                    std::time::Duration::from_secs(2),
                    proxy.set_battery_limit(*percent as u32),
                )
                .await
            }
            _ => return false,
        };

        matches!(res, Ok(Ok(_)))
    })
}

pub fn execute(command: &BatteryCommands) {
    if matches!(command, BatteryCommands::Limit { .. }) && try_call_daemon(command) {
        println!("Request handled by lapctld daemon.");
        return;
    }

    match command {
        BatteryCommands::Limit { percent } => {
            if !(&1..=&100).contains(&percent) {
                error!("Invalid percentage. Please provide a value between 1 and 100.");
                std::process::exit(1);
            }

            println!("Setting battery charge limit to {}%", percent);

            let mut success_count = 0;
            let sys_class_power = Path::new("/sys/class/power_supply");

            if let Ok(entries) = fs::read_dir(sys_class_power) {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();

                    if name_str.starts_with("BAT") {
                        let threshold_path = entry.path().join("charge_control_end_threshold");
                        if threshold_path.exists() {
                            match fs::write(&threshold_path, percent.to_string()) {
                                Ok(_) => {
                                    info!("Successfully set limit for {}", name_str);
                                    success_count += 1;
                                }
                                Err(e) => {
                                    debug!("Failed to write to {}: {}", threshold_path.display(), e)
                                }
                            }
                        } else {
                            // Ideapad fallback
                            let mut ideapad_found = false;
                            if let Ok(ideapad_entries) =
                                fs::read_dir("/sys/bus/platform/drivers/ideapad_acpi")
                            {
                                for ideapad_entry in ideapad_entries.flatten() {
                                    let ideapad_path = ideapad_entry.path();
                                    let conservation_path = ideapad_path.join("conservation_mode");
                                    if conservation_path.exists() {
                                        ideapad_found = true;
                                        let val = if *percent < 100 { "1" } else { "0" };
                                        match fs::write(&conservation_path, val) {
                                            Ok(_) => {
                                                if *percent < 100 {
                                                    println!(
                                                        "WARNING: Your laptop (Lenovo Ideapad) does NOT support custom charge limits (like {}%).",
                                                        percent
                                                    );
                                                    println!(
                                                        "Instead, lapctl has enabled Lenovo's built-in 'Conservation Mode' via the Ideapad ACPI."
                                                    );
                                                    println!(
                                                        "This mode is hard-coded into your laptop's firmware to stop charging at ~60%."
                                                    );
                                                } else {
                                                    println!(
                                                        "Disabled Conservation Mode (charging to 100%)."
                                                    );
                                                }
                                                success_count += 1;
                                            }
                                            Err(e) => debug!(
                                                "Failed to write to conservation_mode: {}",
                                                e
                                            ),
                                        }
                                    }
                                }
                            }
                            if !ideapad_found {
                                debug!("limit not supported for {} (missing thresholds)", name_str);
                            }
                        }
                    }
                }
            }

            if success_count > 0 {
                println!("Operation completed successfully.");
            } else {
                error!(
                    "Hardware does not support dynamic charge thresholds via sysfs or no batteries were found."
                );
            }
        }
        BatteryCommands::Status => {
            // Re-use logic from `lapctl status` to show battery-specific details:
            let sys_class_power = Path::new("/sys/class/power_supply");
            if let Ok(entries) = fs::read_dir(sys_class_power) {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    if name_str.starts_with("BAT") {
                        let bat_path = entry.path();
                        let capacity = fs::read_to_string(bat_path.join("capacity"))
                            .unwrap_or_else(|_| "Unknown".into());
                        let status = fs::read_to_string(bat_path.join("status"))
                            .unwrap_or_else(|_| "Unknown".into());

                        println!("{}:", name_str);
                        println!("  Capacity: {}%", capacity.trim());
                        println!("  Status: {}", status.trim());

                        if let Ok(limit) =
                            fs::read_to_string(bat_path.join("charge_control_end_threshold"))
                        {
                            println!("  Charge Limit: {}%", limit.trim());
                        } else {
                            // Ideapad fallback display
                            if let Ok(ideapad_entries) =
                                fs::read_dir("/sys/bus/platform/drivers/ideapad_acpi")
                            {
                                for ideapad_entry in ideapad_entries.flatten() {
                                    let conservation_path =
                                        ideapad_entry.path().join("conservation_mode");
                                    if conservation_path.exists()
                                        && let Ok(mode) = fs::read_to_string(&conservation_path)
                                    {
                                        if mode.trim() == "1" {
                                            println!("  Charge Limit: Conservation Mode (~60%)");
                                        } else {
                                            println!("  Charge Limit: 100%");
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
