use crate::hardware::gpu;
use std::fs;
use std::path::Path;

pub fn execute() {
    println!("--- lapctl status ---");

    // GPU Mode
    let current_mode = gpu::get_current_mode();
    println!("GPU Mode: {}", current_mode);

    // Battery Limit / Status checks (we can peek into typical battery paths)
    // Note: this assumes a typical sysfs structure. A complete implementation 
    // would parse /sys/class/power_supply/BAT0 or similar.
    let sys_class_power = "/sys/class/power_supply";
    if let Ok(entries) = fs::read_dir(sys_class_power) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with("BAT") {
                let bat_path = entry.path();
                
                let capacity = fs::read_to_string(bat_path.join("capacity")).unwrap_or_else(|_| "Unknown".into());
                let status = fs::read_to_string(bat_path.join("status")).unwrap_or_else(|_| "Unknown".into());
                
                println!("{}:", name_str);
                println!("  Capacity: {}%", capacity.trim());
                println!("  Status: {}", status.trim());
                
                if let Ok(limit) = fs::read_to_string(bat_path.join("charge_control_end_threshold")) {
                    println!("  Charge Limit: {}%", limit.trim());
                } else {
                    if let Ok(ideapad_entries) = fs::read_dir("/sys/bus/platform/drivers/ideapad_acpi") {
                        for ideapad_entry in ideapad_entries.flatten() {
                            let conservation_path = ideapad_entry.path().join("conservation_mode");
                            if conservation_path.exists() {
                                if let Ok(mode) = fs::read_to_string(&conservation_path) {
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

    // Power Profile Status
    if let Ok(profile) = fs::read_to_string("/sys/firmware/acpi/platform_profile") {
        println!("Power Profile: {}", profile.trim());
    } else {
        println!("Power Profile: Unknown");
    }

    // Cooling / Thermal Status
    let mut thermal_found = false;
    if let Ok(ideapad_entries) = fs::read_dir("/sys/bus/platform/drivers/ideapad_acpi") {
        for ideapad_entry in ideapad_entries.flatten() {
            let fan_path = ideapad_entry.path().join("fan_mode");
            if fan_path.exists() {
                if let Ok(mode) = fs::read_to_string(&fan_path) {
                    thermal_found = true;
                    let desc = match mode.trim() {
                        "1" => "Performance",
                        "0" => "Balanced",
                        "2" => "Quiet / Battery Saving",
                        _ => mode.trim()
                    };
                    println!("Cooling Profile: {} (Ideapad)", desc);
                }
            }
        }
    }

    let asus_path = Path::new("/sys/devices/platform/asus-nb-wmi/throttle_thermal_policy");
    if asus_path.exists() {
        if let Ok(mode) = fs::read_to_string(asus_path) {
            thermal_found = true;
            let desc = match mode.trim() {
                "1" => "Performance",
                "0" => "Balanced",
                "2" => "Quiet / Battery Saving",
                _ => mode.trim()
            };
            println!("Cooling Profile: {} (ASUS)", desc);
        }
    }

    if !thermal_found {
        println!("Cooling Profile: Unknown / Firmware managed");
    }
}
