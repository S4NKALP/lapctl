use crate::cli::TouchpadCommands;
use log::{error, info};
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
        let connection = match Connection::system().await {
            Ok(conn) => conn,
            Err(_) => return false,
        };
        let proxy = match LapctlProxy::new(&connection).await {
            Ok(p) => p,
            Err(_) => return false,
        };

        match command {
            TouchpadCommands::Enable => proxy.set_touchpad_inhibition(false).await.is_ok(),
            TouchpadCommands::Disable => proxy.set_touchpad_inhibition(true).await.is_ok(),
        }
    })
}

pub fn execute(command: &TouchpadCommands) {
    if try_call_daemon(command) {
        println!("Request handled by lapctld daemon.");
        return;
    }

    let sys_class_input = Path::new("/sys/class/input");
    if !sys_class_input.exists() {
        error!("Could not find /sys/class/input. Is this a Linux system?");
        return;
    }

    let mut found_touchpads = Vec::new();

    if let Ok(entries) = fs::read_dir(sys_class_input) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name_path = path.join("name");
            let inhibited_path = path.join("inhibited");

            if name_path.exists()
                && inhibited_path.exists()
                && let Ok(name) = fs::read_to_string(&name_path)
                && name.to_lowercase().contains("touchpad")
            {
                found_touchpads.push((name.trim().to_string(), inhibited_path));
            }
        }
    }

    if found_touchpads.is_empty() {
        error!("No touchpad devices found with 'inhibited' support in /sys/class/input.");
        return;
    }

    match command {
        TouchpadCommands::Enable => {
            for (name, path) in found_touchpads {
                match fs::write(&path, "0") {
                    Ok(_) => info!("Enabled touchpad: {}", name),
                    Err(e) => error!("Failed to enable touchpad {}: {}", name, e),
                }
            }
        }
        TouchpadCommands::Disable => {
            for (name, path) in found_touchpads {
                match fs::write(&path, "1") {
                    Ok(_) => info!("Disabled touchpad: {}", name),
                    Err(e) => error!("Failed to disable touchpad {}: {}", name, e),
                }
            }
        }
    }
}
