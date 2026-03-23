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
    match command {
        BatteryCommands::Limit { percent } => {
            if !(&1..=&100).contains(&percent) {
                error!("Invalid percentage. Please provide a value between 1 and 100.");
                std::process::exit(1);
            }
        }
        BatteryCommands::Status => {}
    }

    if matches!(command, BatteryCommands::Limit { .. }) && try_call_daemon(command) {
        return;
    }

    execute_local(command);
}

pub fn execute_local(command: &BatteryCommands) {
    match command {
        BatteryCommands::Limit { percent } => {
            let mut success_count = 0;

            // Ideapad fallback - search for conservation_mode in ideapad_acpi driver directories
            if let Ok(drivers) = fs::read_dir("/sys/bus/platform/drivers/ideapad_acpi") {
                for entry in drivers.flatten() {
                    let conservation_path = entry.path().join("conservation_mode");
                    if conservation_path.exists() {
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
                                    println!("Disabled Conservation Mode (charging to 100%).");
                                }
                                success_count += 1;
                            }
                            Err(e) => {
                                debug!("Failed to write to {}: {}", conservation_path.display(), e)
                            }
                        }
                    }
                }
            }

            // Only try standard thresholds if we haven't already succeeded with Ideapad fallback
            // or if we want to ensure all bases are covered. For now, let's do both but prioritize the message.
            let sys_class_power = Path::new("/sys/class/power_supply");

            if let Ok(entries) = fs::read_dir(sys_class_power) {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();

                    if name_str.starts_with("BAT") {
                        let threshold_path = entry.path().join("charge_control_end_threshold");
                        if threshold_path.exists() {
                            println!(
                                "Setting battery charge limit for {} to {}%",
                                name_str, percent
                            );
                            match fs::write(&threshold_path, percent.to_string()) {
                                Ok(_) => {
                                    info!("Successfully set limit for {}", name_str);
                                    success_count += 1;
                                }
                                Err(e) => {
                                    debug!("Failed to write to {}: {}", threshold_path.display(), e)
                                }
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

                        // Check for Ideapad conservation mode first using robust detection
                        let mut limit_shown = false;
                        if let Ok(drivers) = fs::read_dir("/sys/bus/platform/drivers/ideapad_acpi")
                        {
                            for entry in drivers.flatten() {
                                let conservation_path = entry.path().join("conservation_mode");
                                if conservation_path.exists()
                                    && let Ok(mode) = fs::read_to_string(&conservation_path)
                                {
                                    if mode.trim() == "1" {
                                        println!("  Charge Limit: Conservation Mode (~60%)");
                                        limit_shown = true;
                                    } else if mode.trim() == "0" {
                                        // On Ideapads 0 means 100% or off.
                                    }
                                }
                            }
                        }

                        if !limit_shown {
                            if let Ok(limit) =
                                fs::read_to_string(bat_path.join("charge_control_end_threshold"))
                            {
                                println!("  Charge Limit: {}%", limit.trim());
                            } else {
                                // Re-check if it's an ideapad and we haven't shown a limit (likely 100%)
                                if let Ok(drivers) =
                                    fs::read_dir("/sys/bus/platform/drivers/ideapad_acpi")
                                {
                                    for entry in drivers.flatten() {
                                        if entry.path().join("conservation_mode").exists() {
                                            println!("  Charge Limit: 100%");
                                            break;
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
