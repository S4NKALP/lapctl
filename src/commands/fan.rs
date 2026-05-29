use std::fs;
use std::path::Path;

pub fn execute() {
    println!("--- Fan Speed ---");
    let mut found = false;

    // Read fan speeds from hwmon
    let hwmon_dir = Path::new("/sys/class/hwmon");
    if hwmon_dir.exists()
        && let Ok(entries) = fs::read_dir(hwmon_dir)
    {
        for entry in entries.flatten() {
            let path = entry.path();
            let name_path = path.join("name");
            let Ok(name) = fs::read_to_string(&name_path) else {
                continue;
            };
            let hwmon_name = name.trim().to_string();

            let mut idx = 1;
            loop {
                let input_path = path.join(format!("fan{}_input", idx));
                if !input_path.exists() {
                    break;
                }

                if let Ok(raw) = fs::read_to_string(&input_path) {
                    let rpm: u64 = raw.trim().parse().unwrap_or(0);

                    let label = path.join(format!("fan{}_label", idx));
                    let label_str = fs::read_to_string(&label)
                        .map(|l| l.trim().to_string())
                        .unwrap_or_else(|_| format!("Fan {}", idx));

                    let display_name = match hwmon_name.as_str() {
                        "acpi_fan" => "CPU Fan",
                        other => other,
                    };

                    if rpm == 0 {
                        println!("  {} ({}): Stopped", display_name, label_str);
                    } else {
                        println!("  {} ({}): {} RPM", display_name, label_str, rpm);
                    }
                    found = true;
                }

                idx += 1;
            }
        }
    }

    // Show fan policy from thermal cooling devices
    let thermal_dir = Path::new("/sys/class/thermal");
    if thermal_dir.exists()
        && let Ok(entries) = fs::read_dir(thermal_dir)
    {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if !name_str.starts_with("cooling_device") {
                continue;
            }

            let type_path = entry.path().join("type");
            let state_path = entry.path().join("cur_state");
            let max_state_path = entry.path().join("max_state");

            if let (Ok(type_content), Ok(state), Ok(max_state)) = (
                fs::read_to_string(&type_path),
                fs::read_to_string(&state_path),
                fs::read_to_string(&max_state_path),
            ) {
                let zone_type = type_content.trim();
                let cur_state: u64 = state.trim().parse().unwrap_or(0);
                let max: u64 = max_state.trim().parse().unwrap_or(0);

                if zone_type == "Fan" {
                    let policy = if cur_state == 0 {
                        "Off"
                    } else if cur_state >= max {
                        "Full Speed"
                    } else {
                        "Variable"
                    };
                    println!(
                        "  {} ({}): Level {}/{} ({})",
                        name_str, zone_type, cur_state, max, policy
                    );
                    found = true;
                }
            }
        }
    }

    if !found {
        println!("  No fan sensors found.");
    }
}
