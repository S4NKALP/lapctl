use crate::cli::GpuCommands;
use crate::commands::gpu;
use std::sync::Arc;
use tokio::sync::Mutex;
use zbus::interface;
use zbus::zvariant::OwnedFd;

pub struct LapctlInterface {
    inhibit_fd: Arc<Mutex<Option<OwnedFd>>>,
}

impl Default for LapctlInterface {
    fn default() -> Self {
        Self {
            inhibit_fd: Arc::new(Mutex::new(None)),
        }
    }
}

#[interface(name = "org.lapctl1")]
impl LapctlInterface {
    pub async fn switch_gpu_integrated(&self, no_reboot: bool) -> zbus::fdo::Result<()> {
        gpu::execute_local(&GpuCommands::Integrated { no_reboot });
        Ok(())
    }

    pub async fn switch_gpu_hybrid(
        &self,
        rtd3: i32,
        use_nvidia_current: bool,
        no_reboot: bool,
    ) -> zbus::fdo::Result<()> {
        let rtd3_opt = if rtd3 < 0 { None } else { Some(rtd3 as u8) };
        gpu::execute_local(&GpuCommands::Hybrid {
            rtd3: rtd3_opt,
            use_nvidia_current,
            no_reboot,
        });
        Ok(())
    }

    pub async fn switch_gpu_nvidia(
        &self,
        dm: String,
        force_comp: bool,
        coolbits: i32,
        use_nvidia_current: bool,
        wayland: bool,
        no_reboot: bool,
    ) -> zbus::fdo::Result<()> {
        let dm_opt = if dm.is_empty() { None } else { Some(dm) };
        let coolbits_opt = if coolbits < 0 {
            None
        } else {
            Some(coolbits as u32)
        };
        gpu::execute_local(&GpuCommands::Nvidia {
            dm: dm_opt,
            force_comp,
            coolbits: coolbits_opt,
            use_nvidia_current,
            wayland,
            no_reboot,
        });
        Ok(())
    }

    pub async fn set_battery_limit(&self, percent: u32) -> zbus::fdo::Result<()> {
        crate::commands::battery::execute_local(&crate::cli::BatteryCommands::Limit {
            percent: percent as u8,
        });
        Ok(())
    }

    pub async fn set_power_profile(&self, profile: String) -> zbus::fdo::Result<()> {
        let cmd = match profile.as_str() {
            "performance" => crate::cli::PowerCommands::Performance,
            "balanced" => crate::cli::PowerCommands::Balanced,
            "battery-save" => crate::cli::PowerCommands::BatterySave,
            _ => {
                return Err(zbus::fdo::Error::InvalidArgs(format!(
                    "Invalid power profile: {}",
                    profile
                )));
            }
        };
        crate::commands::power::execute_local(&cmd);
        Ok(())
    }

    pub async fn set_tdp_limit(&self, watts: u32) -> zbus::fdo::Result<()> {
        crate::commands::power::execute_local(&crate::cli::PowerCommands::LimitTdp { watts });
        Ok(())
    }

    pub async fn set_cooling_profile(&self, profile: String) -> zbus::fdo::Result<()> {
        let cmd = match profile.as_str() {
            "performance" => crate::cli::CoolingCommands::Performance,
            "balanced" => crate::cli::CoolingCommands::Balanced,
            "quiet" => crate::cli::CoolingCommands::Quiet,
            _ => {
                return Err(zbus::fdo::Error::InvalidArgs(format!(
                    "Invalid cooling profile: {}",
                    profile
                )));
            }
        };
        crate::commands::cooling::execute_local(&cmd);
        Ok(())
    }

    pub async fn set_touchpad_inhibition(&self, inhibited: bool) -> zbus::fdo::Result<()> {
        let cmd = if inhibited {
            crate::cli::TouchpadCommands::Disable
        } else {
            crate::cli::TouchpadCommands::Enable
        };
        crate::commands::touchpad::execute_local(&cmd);
        Ok(())
    }

    pub async fn set_system_inhibition(
        &self,
        active: bool,
        why: String,
        who: String,
    ) -> zbus::fdo::Result<()> {
        let mut fd_lock = self.inhibit_fd.lock().await;

        if active {
            if fd_lock.is_some() {
                return Ok(()); // Already inhibiting
            }

            let conn = zbus::Connection::system().await.map_err(|e| {
                zbus::fdo::Error::Failed(format!("Failed to connect to system bus: {}", e))
            })?;

            let msg = conn
                .call_method(
                    Some("org.freedesktop.login1"),
                    "/org/freedesktop/login1",
                    Some("org.freedesktop.login1.Manager"),
                    "Inhibit",
                    &("sleep:idle", who, why, "block"),
                )
                .await
                .map_err(|e| zbus::fdo::Error::Failed(format!("Failed to inhibit: {}", e)))?;

            let fd: OwnedFd = msg
                .body()
                .deserialize()
                .map_err(|e| zbus::fdo::Error::Failed(format!("Failed to parse FD: {}", e)))?;

            *fd_lock = Some(fd);
            log::info!("System inhibition activated via lapctld.");
        } else {
            *fd_lock = None;
            log::info!("System inhibition deactivated via lapctld.");
        }

        Ok(())
    }
}
