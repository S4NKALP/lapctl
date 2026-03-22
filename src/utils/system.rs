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
            // Try to re-exec with pkexec
            if let Ok(exe) = std::env::current_exe() {
                let args: Vec<String> = std::env::args().skip(1).collect();
                info!("Not running as root. Attempting to elevate privileges via pkexec...");
                let status = Command::new("pkexec").arg(exe).args(&args).status();

                match status {
                    Ok(s) if s.success() => std::process::exit(0),
                    _ => {
                        error!("Privilege escalation failed or was cancelled.");
                        std::process::exit(1);
                    }
                }
            } else {
                error!(
                    "This operation requires root privileges and could not determine executable path."
                );
                std::process::exit(1);
            }
        }
    }
}

pub fn create_file(path: &str, content: &str, executable: bool) {
    let path_obj = Path::new(path);
    if let Some(parent) = path_obj.parent()
        && !parent.exists()
        && let Err(e) = fs::create_dir_all(parent)
    {
        error!("Failed to create directories for '{}': {}", path, e);
        return;
    }

    match fs::write(path_obj, content) {
        Ok(_) => {
            info!("Created file {}", path);
            debug!("{}", content);

            if executable && let Ok(mut perms) = fs::metadata(path_obj).map(|m| m.permissions()) {
                perms.set_mode(perms.mode() | 0o111);
                let _ = fs::set_permissions(path_obj, perms);
                info!("Added execution privilege to file {}", path);
            }
        }
        Err(e) => {
            error!("Failed to create file '{}': {}", path, e);
        }
    }
}

pub fn get_display_manager() -> Option<String> {
    let output = Command::new("systemctl")
        .args(["show", "-p", "FragmentPath", "display-manager.service"])
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Some(line) = stdout.lines().next()
        && let Some(path_str) = line.strip_prefix("FragmentPath=")
        && !path_str.is_empty()
        && path_str != "/dev/null"
    {
        let dm = Path::new(path_str)
            .file_name()?
            .to_string_lossy()
            .to_string();
        info!("Found {} Display Manager", dm);
        return Some(dm);
    }
    warn!("Display Manager detection is not available");
    None
}

pub fn is_service_active(name: &str) -> bool {
    let output = Command::new("systemctl").args(["is-active", name]).output();
    if let Ok(output) = output {
        return String::from_utf8_lossy(&output.stdout).trim() == "active";
    }
    false
}

pub fn get_active_graphical_sessions() -> Vec<String> {
    let mut sessions = Vec::new();
    let output = Command::new("loginctl")
        .args(["list-sessions", "--no-legend"])
        .output();

    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(id) = parts.first() {
                let show_output = Command::new("loginctl")
                    .args(["show-session", id, "-p", "Type", "-p", "State"])
                    .output();
                if let Ok(show_output) = show_output {
                    let show_stdout = String::from_utf8_lossy(&show_output.stdout);
                    let mut is_graphical = false;
                    let mut is_active = false;
                    for show_line in show_stdout.lines() {
                        if let Some(t) = show_line.strip_prefix("Type=") {
                            if t == "x11" || t == "wayland" {
                                is_graphical = true;
                            }
                        } else if let Some(s) = show_line.strip_prefix("State=")
                            && (s == "active" || s == "online")
                        {
                            is_active = true;
                        }
                    }
                    if is_graphical && is_active {
                        sessions.push(id.to_string());
                    }
                }
            }
        }
    }
    sessions
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
    } else if Path::new("/usr/lib/endeavouros-release").exists()
        && Path::new("/usr/bin/dracut").exists()
    {
        command = vec!["dracut-rebuild"];
    } else if Path::new("/etc/altlinux-release").exists() {
        command = vec!["make-initrd"];
    } else if Path::new("/etc/arch-release").exists() {
        command = vec!["mkinitcpio", "-P"];
    }

    if let Ok(which) = Command::new("which").arg("systemd-inhibit").output()
        && which.status.success()
    {
        let mut new_cmd = vec![
            "systemd-inhibit",
            "--who=lapctl",
            "--why",
            "Rebuilding initramfs",
            "--",
        ];
        new_cmd.extend(command.iter());
        command = new_cmd;
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

pub fn manage_service(name: &str, action: &str) -> Result<(), String> {
    info!("Performing {} on service {}", action, name);
    let mut cmd = Command::new("systemctl");
    cmd.args([action, name]);
    match cmd.status() {
        Ok(s) if s.success() => Ok(()),
        _ => Err(format!("Failed to {} service {}", action, name)),
    }
}

pub fn terminate_session(id: &str) -> Result<(), String> {
    info!("Terminating session {}", id);
    let mut cmd = Command::new("loginctl");
    cmd.args(["terminate-session", id]);
    match cmd.status() {
        Ok(s) if s.success() => Ok(()),
        _ => Err(format!("Failed to terminate session {}", id)),
    }
}
