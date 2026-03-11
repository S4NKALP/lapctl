use crate::cli::PowerCommands;
use crate::utils::system::assert_root;
use log::{error, info};
use std::fs;
use std::path::Path;

fn set_cpu_governor(governor: &str) {
    let cpu_dir = Path::new("/sys/devices/system/cpu");
    if !cpu_dir.exists() {
        return;
    }

    let mut success_count = 0;
    if let Ok(entries) = fs::read_dir(cpu_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if name.starts_with("cpu") && name.chars().nth(3).map_or(false, |c| c.is_ascii_digit()) {
                let scaling_gov_path = path.join("cpufreq/scaling_governor");
                if scaling_gov_path.exists() {
                    match fs::write(&scaling_gov_path, governor) {
                        Ok(_) => success_count += 1,
                        Err(e) => log::debug!("Failed to set CPU governor for {}: {}", name, e),
                    }
                }
            }
        }
    }

    if success_count > 0 {
        info!("Set CPU scaling governor to '{}' on {} cores", governor, success_count);
    } else {
        log::debug!("Could not set CPU scaling governor");
    }
}

fn set_energy_performance_preference(epp: &str) {
    let cpu_dir = Path::new("/sys/devices/system/cpu");
    if !cpu_dir.exists() {
        return;
    }

    let mut success_count = 0;
    if let Ok(entries) = fs::read_dir(cpu_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if name.starts_with("cpu") && name.chars().nth(3).map_or(false, |c| c.is_ascii_digit()) {
                let epp_path = path.join("cpufreq/energy_performance_preference");
                if epp_path.exists() {
                    match fs::write(&epp_path, epp) {
                        Ok(_) => success_count += 1,
                        Err(e) => log::debug!("Failed to set EPP for {}: {}", name, e),
                    }
                }
            }
        }
    }

    if success_count > 0 {
        info!("Set Energy Performance Preference to '{}' on {} cores", epp, success_count);
    }
}

fn set_platform_profile(profile: &str) {
    let platform_profile_path = Path::new("/sys/firmware/acpi/platform_profile");
    if platform_profile_path.exists() {
        match fs::write(platform_profile_path, profile) {
            Ok(_) => info!("Set ACPI platform profile to '{}'", profile),
            Err(e) => error!("Failed to set ACPI platform profile: {}", e),
        }
    } else {
        log::debug!("ACPI platform profile not available on this system");
    }
}

pub fn execute(command: &PowerCommands) {
    assert_root();

    match command {
        PowerCommands::Performance => {
            println!("Setting power profile to Performance");
            set_platform_profile("performance");
            set_cpu_governor("performance");
            set_energy_performance_preference("performance");
            println!("Operation completed successfully.");
        }
        PowerCommands::Balanced => {
            println!("Setting power profile to Balanced");
            set_platform_profile("balanced");
            // Schedutil is the modern balanced default. Fallback isn't critical since cpufreq 
            // usually rejects invalid profiles, and standard Intel/AMD systems use EPP now anyway.
            set_cpu_governor("schedutil"); 
            set_energy_performance_preference("balance_performance");
            println!("Operation completed successfully.");
        }
        PowerCommands::BatterySave => {
            println!("Setting power profile to Battery Saver");
            set_platform_profile("low-power");
            set_cpu_governor("powersave");
            set_energy_performance_preference("power");
            println!("Operation completed successfully.");
        }
    }
}
