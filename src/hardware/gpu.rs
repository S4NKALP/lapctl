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
    let output = Command::new("lspci")
        .output()
        .expect("Failed to execute lspci");
    let lspci_output = String::from_utf8_lossy(&output.stdout);

    for line in lspci_output.lines() {
        if line.contains("NVIDIA")
            && (line.contains("VGA compatible controller") || line.contains("3D controller"))
        {
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

pub fn get_nvidia_gpu_pci_addr() -> Option<String> {
    let output = Command::new("lspci")
        .output()
        .expect("Failed to execute lspci");
    let lspci_output = String::from_utf8_lossy(&output.stdout);

    for line in lspci_output.lines() {
        if line.contains("NVIDIA")
            && (line.contains("VGA compatible controller") || line.contains("3D controller"))
        {
            let pci_addr = line.split_whitespace().next().unwrap();
            if pci_addr.contains(':') {
                if pci_addr.chars().filter(|&c| c == ':').count() == 1 {
                    return Some(format!("0000:{}", pci_addr));
                }
                return Some(pci_addr.to_string());
            }
        }
    }
    None
}

pub fn unbind_gpu(pci_addr: &str) -> Result<(), String> {
    let driver_path = Path::new("/sys/bus/pci/devices")
        .join(pci_addr)
        .join("driver");
    if driver_path.exists() {
        let unbind_path = driver_path.join("unbind");
        info!("Unbinding GPU at {} via {:?}", pci_addr, unbind_path);
        std::fs::write(unbind_path, pci_addr).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn remove_gpu(pci_addr: &str) -> Result<(), String> {
    let remove_path = Path::new("/sys/bus/pci/devices")
        .join(pci_addr)
        .join("remove");
    if remove_path.exists() {
        info!("Removing GPU at {} via {:?}", pci_addr, remove_path);
        std::fs::write(remove_path, "1").map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn rescan_pci() -> Result<(), String> {
    let rescan_path = Path::new("/sys/bus/pci/rescan");
    info!("Rescanning PCI bus via {:?}", rescan_path);
    std::fs::write(rescan_path, "1").map_err(|e| e.to_string())?;
    Ok(())
}

pub fn kill_gpu_processes() -> Result<(), String> {
    info!("Killing processes using NVIDIA GPU...");
    // Try using fuser first as it is more likely to be available and easy to use
    let _ = Command::new("fuser").args(["-k", "/dev/nvidia*"]).output();

    // Also kill specific nvidia services if they are running
    let _ = Command::new("systemctl")
        .args(["stop", "nvidia-persistenced.service"])
        .output();

    Ok(())
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

    let output = Command::new("xrandr")
        .arg("--listproviders")
        .output()
        .ok()?;
    let xrandr_output = String::from_utf8_lossy(&output.stdout);

    let pattern = Regex::new(r"name:.*(ATI|AMD|AMD/ATI)").unwrap();
    if let Some(mat) = pattern.find(&xrandr_output) {
        let matched_str = mat.as_str();
        return Some(matched_str[5..].trim().to_string());
    }

    warn!("Could not find AMD iGPU in 'xrandr' output.");
    None
}
