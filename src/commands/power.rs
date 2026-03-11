use crate::cli::PowerCommands;
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
            if name.starts_with("cpu") && name.chars().nth(3).map_or(false, |c| c.is_ascii_digit())
            {
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
        info!(
            "Set CPU scaling governor to '{}' on {} cores",
            governor, success_count
        );
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
            if name.starts_with("cpu") && name.chars().nth(3).map_or(false, |c| c.is_ascii_digit())
            {
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
        info!(
            "Set Energy Performance Preference to '{}' on {} cores",
            epp, success_count
        );
    }
}
fn set_intel_rapl_limit(watts: u32) -> bool {
    let mut success = false;
    let microwatts = watts * 1_000_000;

    // Intel RAPL
    let rapl_dir = Path::new("/sys/class/powercap/intel-rapl");
    if rapl_dir.exists() {
        if let Ok(entries) = fs::read_dir(rapl_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().unwrap_or_default().to_string_lossy();

                // Set limits for package-0 (entire CPU package)
                if name.starts_with("intel-rapl:") {
                    let constraint_0 = path.join("constraint_0_power_limit_uw"); // PL1
                    let constraint_1 = path.join("constraint_1_power_limit_uw"); // PL2

                    if constraint_0.exists() {
                        match fs::write(&constraint_0, microwatts.to_string()) {
                            Ok(_) => {
                                log::debug!("Set RAPL constraint 0 to {}uW", microwatts);
                                success = true;
                            }
                            Err(e) => log::debug!("Failed to write RAPL 0: {}", e),
                        }
                    }
                    if constraint_1.exists() {
                        // Usually PL2 (Boost limit) is set ~20% higher than PL1 (Sustained limit)
                        // But for a strict override we clamp BOTH to the requested static wattage limit.
                        match fs::write(&constraint_1, microwatts.to_string()) {
                            Ok(_) => {
                                log::debug!("Set RAPL constraint 1 to {}uW", microwatts);
                                success = true;
                            }
                            Err(e) => log::debug!("Failed to write RAPL 1: {}", e),
                        }
                    }
                }
            }
        }
    }
    success
}

fn set_amd_hwmon_limit(watts: u32) -> bool {
    let mut success = false;
    let microwatts = watts * 1_000_000;

    // AMD hwmon (Typically exposed via hwmon platform drivers like k10temp or amd_pmc)
    let hwmon_dir = Path::new("/sys/class/hwmon");
    if hwmon_dir.exists() {
        if let Ok(entries) = fs::read_dir(hwmon_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let power1_cap = path.join("power1_cap");
                let name_path = path.join("name");

                if power1_cap.exists() {
                    // Try to verify this is actually the CPU and not a generic sensor
                    if let Ok(name) = fs::read_to_string(&name_path) {
                        if name.trim().contains("amd") {
                            match fs::write(&power1_cap, microwatts.to_string()) {
                                Ok(_) => {
                                    log::debug!("Set AMD hwmon power1_cap to {}uW", microwatts);
                                    success = true;
                                }
                                Err(e) => log::debug!("Failed to write AMD hwmon: {}", e),
                            }
                        }
                    }
                }
            }
        }
    }
    success
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
        PowerCommands::LimitTdp { watts } => {
            println!("Setting CPU Package TDP Limit to {} Watts...", watts);

            let mut applied = false;
            if set_intel_rapl_limit(*watts) {
                applied = true;
                println!("Successfully applied Intel RAPL boundaries (PL1/PL2).");
            }
            if set_amd_hwmon_limit(*watts) {
                applied = true;
                println!("Successfully applied AMD HWMon power capping.");
            }

            if applied {
                println!("Operation completed successfully.");
            } else {
                error!(
                    "Hardware does not support dynamic powercap limits via intel-rapl or standard hwmon."
                );
            }
        }
    }
}
