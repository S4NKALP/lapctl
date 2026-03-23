use crate::cli::TouchpadCommands;
use log::{debug, error, info};
use std::fs;
use std::path::Path;
use zbus::Connection;
use zbus::proxy;

#[proxy(
    interface = "org.lapctl1",
    default_service = "org.lapctl",
    default_path = "/org/lapctl"
)]
trait Lapctl {
    async fn set_touchpad_inhibition(&self, inhibited: bool) -> zbus::Result<()>;
}

fn try_call_daemon(command: &TouchpadCommands) -> bool {
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return false,
    };

    rt.block_on(async {
        let connection =
            match tokio::time::timeout(std::time::Duration::from_secs(2), Connection::system())
                .await
            {
                Ok(Ok(conn)) => conn,
                _ => return false,
            };

        let proxy = match LapctlProxy::new(&connection).await {
            Ok(p) => p,
            Err(_) => return false,
        };

        let res = match command {
            TouchpadCommands::Enable => {
                tokio::time::timeout(
                    std::time::Duration::from_secs(2),
                    proxy.set_touchpad_inhibition(false),
                )
                .await
            }
            TouchpadCommands::Disable => {
                tokio::time::timeout(
                    std::time::Duration::from_secs(2),
                    proxy.set_touchpad_inhibition(true),
                )
                .await
            }
        };

        matches!(res, Ok(Ok(_)))
    })
}

pub fn execute(command: &TouchpadCommands) {
    match command {
        TouchpadCommands::Enable => println!("Enabling touchpad..."),
        TouchpadCommands::Disable => println!("Disabling touchpad..."),
    }

    if try_call_daemon(command) {
        return;
    }

    execute_local(command);
}

pub fn execute_local(command: &TouchpadCommands) {
    match command {
        TouchpadCommands::Enable => set_touchpad_inhibition(false),
        TouchpadCommands::Disable => set_touchpad_inhibition(true),
    }
}

fn set_touchpad_inhibition(inhibited: bool) {
    let dev_input = Path::new("/sys/class/input");
    if !dev_input.exists() {
        error!("Could not find /sys/class/input. Is this a Linux system?");
        return;
    }

    if let Ok(entries) = fs::read_dir(dev_input) {
        let mut found = false;
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if name_str.starts_with("input") {
                let name_file = entry.path().join("name");
                if let Ok(content) = fs::read_to_string(name_file) {
                    let content_lower = content.to_lowercase();
                    if content_lower.contains("touchpad") || content_lower.contains("trackpad") {
                        found = true;
                        let inhibited_file = entry.path().join("inhibited");
                        if inhibited_file.exists() {
                            let val = if inhibited { "1" } else { "0" };
                            match fs::write(&inhibited_file, val) {
                                Ok(_) => info!("Set inhibit={} for {}", inhibited, name_str),
                                Err(e) => {
                                    debug!("Failed to write to {}: {}", inhibited_file.display(), e)
                                }
                            }
                        }
                    }
                }
            }
        }
        if found {
            println!("Operation completed successfully.");
        } else {
            error!("No touchpad device found in /sys/class/input.");
        }
    } else {
        error!("Failed to read directory /sys/class/input.");
    }
}
