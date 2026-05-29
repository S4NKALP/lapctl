use log::debug;
use std::fs;
use std::path::Path;

pub fn execute() {
    println!("--- Temperature ---");
    let mut found = false;

    // Read all hwmon temperature sensors
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
                let input_path = path.join(format!("temp{}_input", idx));
                if !input_path.exists() {
                    break;
                }

                if let Ok(raw) = fs::read_to_string(&input_path) {
                    let millideg: i64 = raw.trim().parse().unwrap_or(0);
                    let celsius = millideg as f64 / 1000.0;

                    let label = path.join(format!("temp{}_label", idx));
                    let label_str = fs::read_to_string(&label)
                        .map(|l| l.trim().to_string())
                        .unwrap_or_else(|_| format!("Sensor {}", idx));

                    let display_name = match hwmon_name.as_str() {
                        "k10temp" => "CPU",
                        "acpitz" => "ACPI Thermal Zone",
                        "nvme" => "NVMe SSD",
                        "mt7921_phy0" => "WiFi",
                        other => other,
                    };

                    println!(
                        "  {} ({}): {:.1}°C",
                        display_name, label_str, celsius
                    );
                    found = true;
                }

                idx += 1;
            }
        }
    }

    // NVIDIA GPU temperature via nvidia-smi
    if let Ok(output) = std::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=temperature.gpu,power.draw,clocks.current.graphics",
            "--format=csv,noheader",
        ])
        .output()
        && output.status.success()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split(", ").collect();
            if let Some(temp) = parts.first()
                && let Ok(temp_c) = temp.trim().parse::<f64>()
            {
                let info = if parts.len() >= 3 {
                    format!(
                        " [{} @ {}]",
                        parts[1].trim(),
                        parts[2].trim()
                    )
                } else {
                    String::new()
                };
                println!("  GPU (NVIDIA): {:.0}°C{}", temp_c, info);
                found = true;
            }
        }
    } else {
        debug!("nvidia-smi not available, skipping GPU temperature");
    }

    // Read thermal zone temperatures
    let thermal_dir = Path::new("/sys/class/thermal");
    if thermal_dir.exists()
        && let Ok(entries) = fs::read_dir(thermal_dir)
    {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if !name_str.starts_with("thermal_zone") {
                continue;
            }

            let type_path = entry.path().join("type");
            let temp_path = entry.path().join("temp");

            if let (Ok(type_content), Ok(temp_content)) =
                (fs::read_to_string(&type_path), fs::read_to_string(&temp_path))
            {
                let zone_type = type_content.trim();
                let millideg: i64 = temp_content.trim().parse().unwrap_or(0);
                let celsius = millideg as f64 / 1000.0;

                // Skip if we already showed this from hwmon
                if zone_type == "x86_pkg_temp"
                    || zone_type == "k10temp"
                    || zone_type == "acpitz"
                {
                    continue;
                }

                println!("  {} ({}): {:.1}°C", name_str, zone_type, celsius);
                found = true;
            }
        }
    }

    if !found {
        println!("  No temperature sensors found.");
    }
}
