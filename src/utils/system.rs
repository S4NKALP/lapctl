use log::{debug, error, info, warn};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

pub fn assert_root() {
    let output = Command::new("id").arg("-u").output();
    if let Ok(output) = output {
        let is_root = String::from_utf8_lossy(&output.stdout).trim() == "0";
        if !is_root {
            error!("This operation requires root privileges");
            std::process::exit(1);
        }
    }
}

pub fn create_file(path: &str, content: &str, executable: bool) {
    let path_obj = Path::new(path);
    if let Some(parent) = path_obj.parent() {
        if !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                error!("Failed to create directories for '{}': {}", path, e);
                return;
            }
        }
    }

    match fs::write(path_obj, content) {
        Ok(_) => {
            info!("Created file {}", path);
            debug!("{}", content);

            if executable {
                if let Ok(mut perms) = fs::metadata(path_obj).map(|m| m.permissions()) {
                    perms.set_mode(perms.mode() | 0o111);
                    let _ = fs::set_permissions(path_obj, perms);
                    info!("Added execution privilege to file {}", path);
                }
            }
        }
        Err(e) => {
            error!("Failed to create file '{}': {}", path, e);
        }
    }
}

pub fn get_display_manager() -> Option<String> {
    if let Ok(content) = fs::read_to_string("/etc/systemd/system/display-manager.service") {
        let re = regex::Regex::new(r"ExecStart=(.+)\n").unwrap();
        if let Some(cap) = re.captures(&content) {
            let path = cap.get(1).unwrap().as_str();
            let dm = Path::new(path).file_name()?.to_string_lossy().to_string();
            info!("Found {} Display Manager", dm);
            return Some(dm);
        }
    }
    warn!("Display Manager detection is not available");
    None
}

pub fn rebuild_initramfs() {
    let mut command: Vec<&str> = Vec::new();

    if Path::new("/ostree").exists() || Path::new("/sysroot/ostree").exists() {
        println!("Rebuilding the initramfs with rpm-ostree...");
        command = vec!["rpm-ostree", "initramfs", "--enable", "--arg=--force"];
    } else if Path::new("/etc/debian_version").exists() {
        command = vec!["update-initramfs", "-u", "-k", "all"];
    } else if Path::new("/etc/redhat-release").exists() || Path::new("/usr/bin/zypper").exists() {
        command = vec!["dracut", "--force", "--regenerate-all"];
    } else if Path::new("/usr/lib/endeavouros-release").exists() && Path::new("/usr/bin/dracut").exists() {
        command = vec!["dracut-rebuild"];
    } else if Path::new("/etc/altlinux-release").exists() {
        command = vec!["make-initrd"];
    } else if Path::new("/etc/arch-release").exists() {
        command = vec!["mkinitcpio", "-P"];
    }

    if let Ok(which) = Command::new("which").arg("systemd-inhibit").output() {
        if which.status.success() {
            let mut new_cmd = vec!["systemd-inhibit", "--who=lapctl", "--why", "Rebuilding initramfs", "--"];
            new_cmd.extend(command.iter());
            command = new_cmd;
        }
    }

    if !command.is_empty() {
        println!("Rebuilding the initramfs...");
        let is_debug = log::log_enabled!(log::Level::Debug);
        
        let mut cmd = Command::new(command[0]);
        cmd.args(&command[1..]);

        if !is_debug {
            cmd.stdout(std::process::Stdio::null());
            cmd.stderr(std::process::Stdio::null());
        }

        match cmd.status() {
            Ok(status) if status.success() => {
                println!("Successfully rebuilt the initramfs!");
            }
            _ => {
                error!("An error occurred while rebuilding the initramfs");
            }
        }
    }
}
