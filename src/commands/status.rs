use crate::hardware::gpu;
use std::fs;

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
                
                // Print charing limit if exists (e.g., charge_control_end_threshold)
                if let Ok(limit) = fs::read_to_string(bat_path.join("charge_control_end_threshold")) {
                    println!("  Charge Limit: {}%", limit.trim());
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
}
