use crate::cli::CoolingCommands;
use log::{debug, info};
use std::fs;
use std::path::Path;

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
