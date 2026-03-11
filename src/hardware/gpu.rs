use log::{error, info, warn};
use regex::Regex;
use std::path::Path;
use std::process::Command;

pub fn get_current_mode() -> String {
    let blacklist_path = Path::new("/etc/modprobe.d/blacklist-nvidia.conf");
    let udev_integrated_path = Path::new("/etc/udev/rules.d/50-remove-nvidia.rules");
    let old_udev_path = Path::new("/lib/udev/rules.d/50-remove-nvidia.rules");
    let xorg_path = Path::new("/etc/X11/xorg.conf");
    let wayland_marker = Path::new("/etc/lapctl-wayland-nvidia");
    let modeset_path = Path::new("/etc/modprobe.d/nvidia.conf");

    if blacklist_path.exists() && (udev_integrated_path.exists() || old_udev_path.exists()) {
        "integrated".to_string()
    } else if (xorg_path.exists() || wayland_marker.exists()) && modeset_path.exists() {
        "nvidia".to_string()
    } else {
        "hybrid".to_string()
    }
}

pub fn get_nvidia_gpu_pci_bus() -> String {
    let output = Command::new("lspci").output().expect("Failed to execute lspci");
    let lspci_output = String::from_utf8_lossy(&output.stdout);

    for line in lspci_output.lines() {
        if line.contains("NVIDIA") && (line.contains("VGA compatible controller") || line.contains("3D controller")) {
            let pci_bus_id_raw = line.split_whitespace().next().unwrap();
            let pci_bus_id = pci_bus_id_raw.replace("0000:", "");
            info!("Found Nvidia GPU at {}", pci_bus_id);

            let parts: Vec<&str> = pci_bus_id.split(':').collect();
            let bus = parts[0];
            let device_fun: Vec<&str> = parts[1].split('.').collect();
            let device = device_fun[0];
            let function = device_fun[1];

            return format!(
                "PCI:{}:{}:{}",
                u32::from_str_radix(bus, 16).unwrap_or(0),
                u32::from_str_radix(device, 16).unwrap_or(0),
                u32::from_str_radix(function, 16).unwrap_or(0)
            );
        }
    }

    error!("Could not find Nvidia GPU");
    println!("Try switching to hybrid mode first!");
    std::process::exit(1);
}

pub fn get_igpu_vendor() -> Option<String> {
    let output = Command::new("lspci").output().ok()?;
    let lspci_output = String::from_utf8_lossy(&output.stdout);

    for line in lspci_output.lines() {
        if line.contains("VGA compatible controller") || line.contains("Display controller") {
            if line.contains("Intel") {
                info!("Found Intel iGPU");
                return Some("intel".to_string());
            } else if line.contains("ATI") || line.contains("AMD") {
                info!("Found AMD iGPU");
                return Some("amd".to_string());
            }
        }
    }
    warn!("Could not find Intel or AMD iGPU");
    None
}

pub fn get_amd_igpu_name() -> Option<String> {
    if !Path::new("/usr/bin/xrandr").exists() {
        warn!("The 'xrandr' command is not available. Make sure the package is installed!");
        return None;
    }

    let output = Command::new("xrandr").arg("--listproviders").output().ok()?;
    let xrandr_output = String::from_utf8_lossy(&output.stdout);

    let pattern = Regex::new(r"name:.*(ATI|AMD|AMD/ATI)").unwrap();
    if let Some(mat) = pattern.find(&xrandr_output) {
        let matched_str = mat.as_str();
        return Some(matched_str[5..].trim().to_string());
    }

    warn!("Could not find AMD iGPU in 'xrandr' output.");
    None
}
