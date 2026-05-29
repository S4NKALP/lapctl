use crate::cli::GpuCommands;
use crate::hardware::gpu::{
    get_amd_igpu_name, get_current_mode, get_igpu_vendor, get_nvidia_gpu_pci_addr,
    get_nvidia_gpu_pci_bus, kill_gpu_processes, remove_gpu, rescan_pci, unbind_gpu,
};
use crate::utils::system::{
    assert_root, create_file, get_active_graphical_sessions, get_display_manager,
    is_service_active, manage_service, rebuild_initramfs, terminate_session,
};
use fs2::FileExt;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use zbus::Connection;
use zbus::proxy;

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

fn setup_signal_handler() {
    if let Err(e) = ctrlc::set_handler(|| {
        INTERRUPTED.store(true, Ordering::SeqCst);
    }) {
        log::debug!("Could not set Ctrl+C handler (may already be set): {}", e);
    }
}

struct DmGuard {
    name: Option<String>,
}

impl Drop for DmGuard {
    fn drop(&mut self) {
        if let Some(ref name) = self.name {
            info!("Restarting display manager: {}", name);
            if let Err(e) = manage_service(name, "start") {
                error!("Failed to restart display manager '{}': {}", name, e);
            }
        }
    }
}

const LOCK_PATH: &str = "/var/lock/lapctl.lock";

struct LockGuard;

impl LockGuard {
    fn acquire() -> Result<Self, String> {
        let file = fs::File::create(LOCK_PATH)
            .map_err(|e| format!("Failed to create lock file: {}", e))?;
        file.try_lock_exclusive()
            .map_err(|e| format!("Another GPU switch operation is already in progress: {}", e))?;
        info!("Acquired exclusive lock");
        Ok(LockGuard)
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        if let Err(e) = fs::remove_file(LOCK_PATH) {
            log::debug!("Failed to remove lock file: {}", e);
        }
        info!("Released lock");
    }
}

#[proxy(
    interface = "org.lapctl1",
    default_service = "org.lapctl",
    default_path = "/org/lapctl"
)]
trait Lapctl {
    async fn switch_gpu_integrated(&self, no_reboot: bool) -> zbus::Result<()>;
    async fn switch_gpu_hybrid(
        &self,
        rtd3: i32,
        use_nvidia_current: bool,
        no_reboot: bool,
    ) -> zbus::Result<()>;
    async fn switch_gpu_nvidia(
        &self,
        dm: String,
        force_comp: bool,
        coolbits: i32,
        use_nvidia_current: bool,
        wayland: bool,
        no_reboot: bool,
    ) -> zbus::Result<()>;
}

fn try_call_daemon(cmd: &GpuCommands) -> bool {
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

        let res = match cmd {
            GpuCommands::Integrated { no_reboot } => {
                tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    proxy.switch_gpu_integrated(*no_reboot),
                )
                .await
            }
            GpuCommands::Hybrid {
                rtd3,
                use_nvidia_current,
                no_reboot,
            } => {
                let rtd3_val = rtd3.map(|v| v as i32).unwrap_or(-1);
                tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    proxy.switch_gpu_hybrid(rtd3_val, *use_nvidia_current, *no_reboot),
                )
                .await
            }
            GpuCommands::Nvidia {
                dm,
                force_comp,
                coolbits,
                use_nvidia_current,
                wayland,
                no_reboot,
            } => {
                let dm_val = dm.clone().unwrap_or_default();
                let coolbits_val = coolbits.map(|v| v as i32).unwrap_or(-1);
                tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    proxy.switch_gpu_nvidia(
                        dm_val,
                        *force_comp,
                        coolbits_val,
                        *use_nvidia_current,
                        *wayland,
                        *no_reboot,
                    ),
                )
                .await
            }
            _ => return false,
        };

        matches!(res, Ok(Ok(_)))
    })
}

fn is_wayland_session() -> bool {
    std::env::var("WAYLAND_DISPLAY").is_ok()
        || std::env::var("XDG_SESSION_TYPE")
            .map(|v| v == "wayland")
            .unwrap_or(false)
}

const CACHE_FILE_PATH: &str = "/var/cache/lapctl/cache.json";
const BLACKLIST_PATH: &str = "/etc/modprobe.d/blacklist-nvidia.conf";
const BLACKLIST_CONTENT: &str = r#"# Automatically generated by lapctl
blacklist nouveau
blacklist nova_core
blacklist nova_drm
blacklist nvidia
blacklist nvidia_drm
blacklist nvidia_uvm
blacklist nvidia_modeset
blacklist nvidia_current
blacklist nvidia_current_drm
blacklist nvidia_current_uvm
blacklist nvidia_current_modeset
blacklist i2c_nvidia_gpu
alias nouveau off
alias nova_core off
alias nova_drm off
alias nvidia off
alias nvidia_drm off
alias nvidia_uvm off
alias nvidia_modeset off
alias nvidia_current off
alias nvidia_current_drm off
alias nvidia_current_uvm off
alias nvidia_current_modeset off
alias i2c_nvidia_gpu off
"#;

const UDEV_INTEGRATED_PATH: &str = "/etc/udev/rules.d/50-remove-nvidia.rules";
const UDEV_INTEGRATED: &str = r#"# Automatically generated by lapctl

# Remove NVIDIA USB xHCI Host Controller devices, if present
ACTION=="add", SUBSYSTEM=="pci", ATTR{vendor}=="0x10de", ATTR{class}=="0x0c0330", ATTR{power/control}="auto", ATTR{remove}="1"

# Remove NVIDIA USB Type-C UCSI devices, if present
ACTION=="add", SUBSYSTEM=="pci", ATTR{vendor}=="0x10de", ATTR{class}=="0x0c8000", ATTR{power/control}="auto", ATTR{remove}="1"

# Remove NVIDIA Audio devices, if present
ACTION=="add", SUBSYSTEM=="pci", ATTR{vendor}=="0x10de", ATTR{class}=="0x040300", ATTR{power/control}="auto", ATTR{remove}="1"

# Remove NVIDIA VGA/3D controller devices
ACTION=="add", SUBSYSTEM=="pci", ATTR{vendor}=="0x10de", ATTR{class}=="0x03[0-9]*", ATTR{power/control}="auto", ATTR{remove}="1"
"#;

const UDEV_PM_PATH: &str = "/etc/udev/rules.d/80-nvidia-pm.rules";
const UDEV_PM_CONTENT: &str = r#"# Automatically generated by lapctl

# Remove NVIDIA USB xHCI Host Controller devices, if present
ACTION=="add", SUBSYSTEM=="pci", ATTR{vendor}=="0x10de", ATTR{class}=="0x0c0330", ATTR{remove}="1"

# Remove NVIDIA USB Type-C UCSI devices, if present
ACTION=="add", SUBSYSTEM=="pci", ATTR{vendor}=="0x10de", ATTR{class}=="0x0c8000", ATTR{remove}="1"

# Remove NVIDIA Audio devices, if present
ACTION=="add", SUBSYSTEM=="pci", ATTR{vendor}=="0x10de", ATTR{class}=="0x040300", ATTR{remove}="1"

# Enable runtime PM for NVIDIA VGA/3D controller devices on driver bind
ACTION=="bind", SUBSYSTEM=="pci", ATTR{vendor}=="0x10de", ATTR{class}=="0x030000", TEST=="power/control", ATTR{power/control}="auto"
ACTION=="bind", SUBSYSTEM=="pci", ATTR{vendor}=="0x10de", ATTR{class}=="0x030200", TEST=="power/control", ATTR{power/control}="auto"

# Disable runtime PM for NVIDIA VGA/3D controller devices on driver unbind
ACTION=="unbind", SUBSYSTEM=="pci", ATTR{vendor}=="0x10de", ATTR{class}=="0x030000", TEST=="power/control", ATTR{power/control}="on"
ACTION=="unbind", SUBSYSTEM=="pci", ATTR{vendor}=="0x10de", ATTR{class}=="0x030200", TEST=="power/control", ATTR{power/control}="on"
"#;

const XORG_PATH: &str = "/etc/X11/xorg.conf";
const EXTRA_XORG_PATH: &str = "/etc/X11/xorg.conf.d/10-nvidia.conf";
const MODESET_PATH: &str = "/etc/modprobe.d/nvidia.conf";
const SDDM_XSETUP_PATH: &str = "/usr/share/sddm/scripts/Xsetup";
const LIGHTDM_SCRIPT_PATH: &str = "/etc/lightdm/nvidia.sh";
const LIGHTDM_CONFIG_PATH: &str = "/etc/lightdm/lightdm.conf.d/20-nvidia.conf";

const SDDM_XSETUP_CONTENT: &str = r#"#!/bin/sh
# Xsetup - run as root before the login dialog appears

"#;

const EXTRA_XORG_CONTENT: &str = r#"# Automatically generated by lapctl

Section "OutputClass"
    Identifier "nvidia"
    MatchDriver "nvidia-drm"
    Driver "nvidia"
"#;

const LIGHTDM_CONFIG_CONTENT: &str = r#"# Automatically generated by lapctl

[Seat:*]
display-setup-script=/etc/lightdm/nvidia.sh
"#;

const NVIDIA_XRANDR_SCRIPT: &str = r#"#!/bin/sh
# Automatically generated by lapctl

current=""

xrandr --setprovideroutputsource "{}" NVIDIA-0
xrandr --auto

for next in $(xrandr --listmonitors | grep -E " *[0-9]+:.*" | cut -d" " -f6); do
  [ -z "$current" ] && current=$next && continue
  xrandr --output "$current" --auto --output "$next" --auto --right-of "$current"
  current=$next
done
"#;

// Helpers
pub fn xorg_intel(bus_id: &str) -> String {
    format!(
        r#"# Automatically generated by lapctl

Section "ServerLayout"
    Identifier "layout"
    Screen 0 "nvidia"
    Inactive "intel"
EndSection

Section "Device"
    Identifier "nvidia"
    Driver "nvidia"
    BusID "{}"
EndSection

Section "Screen"
    Identifier "nvidia"
    Device "nvidia"
    Option "AllowEmptyInitialConfiguration"
EndSection

Section "Device"
    Identifier "intel"
    Driver "modesetting"
EndSection

Section "Screen"
    Identifier "intel"
    Device "intel"
EndSection
"#,
        bus_id
    )
}

pub fn xorg_amd(bus_id: &str) -> String {
    format!(
        r#"# Automatically generated by lapctl

Section "ServerLayout"
    Identifier "layout"
    Screen 0 "nvidia"
    Inactive "amdgpu"
EndSection

Section "Device"
    Identifier "nvidia"
    Driver "nvidia"
    BusID "{}"
EndSection

Section "Screen"
    Identifier "nvidia"
    Device "nvidia"
    Option "AllowEmptyInitialConfiguration"
EndSection

Section "Device"
    Identifier "amdgpu"
    Driver "amdgpu"
EndSection

Section "Screen"
    Identifier "amd"
    Device "amdgpu"
EndSection
"#,
        bus_id
    )
}

pub fn xorg_modesetting(bus_id: &str) -> String {
    format!(
        r#"# Automatically generated by lapctl

Section "ServerLayout"
    Identifier "layout"
    Screen 0 "nvidia"
    Inactive "modesetting"
EndSection

Section "Device"
    Identifier "nvidia"
    Driver "nvidia"
    BusID "{}"
EndSection

Section "Screen"
    Identifier "nvidia"
    Device "nvidia"
    Option "AllowEmptyInitialConfiguration"
EndSection

Section "Device"
    Identifier "modesetting"
    Driver "modesetting"
EndSection

Section "Screen"
    Identifier "modesetting"
    Device "modesetting"
EndSection
"#,
        bus_id
    )
}

pub fn generate_xrandr_script(igpu_vendor: Option<&String>) -> String {
    let source_name = match igpu_vendor.map(|s| s.as_str()) {
        Some("intel") => "modesetting".to_string(),
        Some("amd") => {
            if let Some(amd_name) = get_amd_igpu_name() {
                amd_name
            } else {
                "modesetting".to_string()
            }
        }
        _ => "modesetting".to_string(),
    };
    NVIDIA_XRANDR_SCRIPT.replace("{}", &source_name)
}

fn cleanup() {
    let to_remove = vec![
        BLACKLIST_PATH,
        UDEV_INTEGRATED_PATH,
        UDEV_PM_PATH,
        XORG_PATH,
        EXTRA_XORG_PATH,
        MODESET_PATH,
        LIGHTDM_SCRIPT_PATH,
        LIGHTDM_CONFIG_PATH,
        "/etc/lapctl-wayland-nvidia",
        // legacy paths
        "/etc/X11/xorg.conf.d/90-nvidia.conf",
        "/lib/udev/rules.d/50-remove-nvidia.rules",
        "/lib/udev/rules.d/80-nvidia-pm.rules",
    ];

    for file_path in to_remove {
        if Path::new(file_path).exists() {
            if let Err(e) = fs::remove_file(file_path) {
                error!("Failed to remove file '{}': {}", file_path, e);
            } else {
                info!("Removed file {}", file_path);
            }
        }
    }

    let backup_path = format!("{}.bak", SDDM_XSETUP_PATH);
    if Path::new(&backup_path).exists() {
        info!("Restoring Xsetup backup");
        if let Ok(content) = fs::read_to_string(&backup_path) {
            create_file(SDDM_XSETUP_PATH, &content, true);
        }
        if let Err(e) = fs::remove_file(&backup_path) {
            error!("Failed to remove backup: {}", e);
        } else {
            info!("Removed file {}", backup_path);
        }
    }
}

// CACHING LOGIC
#[derive(Serialize, Deserialize, Debug)]
struct Cache {
    nvidia_gpu_pci_bus: String,
}

fn create_cache_obj(bus_id: String) -> Cache {
    Cache {
        nvidia_gpu_pci_bus: bus_id,
    }
}

fn create_cache_file() {
    let pci_addr = get_nvidia_gpu_pci_addr();
    if pci_addr.is_none() {
        log::debug!("NVIDIA GPU not found, skipping cache creation");
        return;
    }

    let bus_id = get_nvidia_gpu_pci_bus();
    let cache = create_cache_obj(bus_id);

    if let Some(parent) = Path::new(CACHE_FILE_PATH).parent()
        && let Err(e) = fs::create_dir_all(parent)
    {
        log::debug!("Failed to create cache directory: {}", e);
    }

    if let Ok(json) = serde_json::to_string_pretty(&cache) {
        let tmp_path = format!("{}.tmp", CACHE_FILE_PATH);
        if fs::write(&tmp_path, &json).is_ok() {
            if fs::rename(&tmp_path, CACHE_FILE_PATH).is_ok() {
                log::debug!("Created/Updated cache file {}", CACHE_FILE_PATH);
            } else {
                if let Err(e) = fs::remove_file(&tmp_path) {
                    log::debug!("Failed to remove stale temp cache file: {}", e);
                }
                log::debug!("Failed to rename temp cache file");
            }
        }
    }
}

fn read_cache_file() -> Result<Cache, String> {
    if Path::new(CACHE_FILE_PATH).exists() {
        let content = fs::read_to_string(CACHE_FILE_PATH).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())
    } else if get_current_mode() == "hybrid" {
        Ok(create_cache_obj(get_nvidia_gpu_pci_bus()))
    } else {
        Err(
            "No cache present. Operation requires that the system be in the hybrid Optimus mode"
                .into(),
        )
    }
}

fn prepare_no_reboot(dm_flag: Option<String>) -> Result<DmGuard, String> {
    setup_signal_handler();
    let dm_name = dm_flag.or_else(get_display_manager);

    if let Some(ref name) = dm_name
        && is_service_active(name)
    {
        info!("Stopping display manager: {}", name);
        manage_service(name, "stop")?;
        return Ok(DmGuard {
            name: Some(name.clone()),
        });
    }

    // If no active DM found, check for graphical sessions
    let sessions = get_active_graphical_sessions();
    if !sessions.is_empty() {
        println!("No active display manager found, but graphical sessions are running.");
        println!("Terminating graphical sessions for no-reboot GPU switching...");
        for session_id in sessions {
            if let Err(e) = terminate_session(&session_id) {
                error!("Failed to terminate session {}: {}", session_id, e);
            }
        }
        // Give some time for sessions to terminate
        std::thread::sleep(std::time::Duration::from_secs(2));
    }

    Ok(DmGuard { name: None })
}

fn switch_integrated(no_reboot: bool) {
    assert_root();
    let _lock = match LockGuard::acquire() {
        Ok(l) => l,
        Err(e) => {
            error!("{}", e);
            return;
        }
    };
    println!("Switching to integrated mode");

    if !Path::new(CACHE_FILE_PATH).exists() {
        create_cache_file();
    }

    cleanup();

    create_file(BLACKLIST_PATH, BLACKLIST_CONTENT, false);
    create_file(UDEV_INTEGRATED_PATH, UDEV_INTEGRATED, false);

    let _guard = if no_reboot {
        let guard = match prepare_no_reboot(None) {
            Ok(g) => g,
            Err(e) => {
                error!("{}", e);
                return;
            }
        };

        if let Err(e) = kill_gpu_processes() {
            error!("Failed to kill GPU processes: {}", e);
        }

        // Try to unload modules
        let modules = ["nvidia_uvm", "nvidia_modeset", "nvidia_drm", "nvidia"];
        for module in modules {
            if let Err(e) = Command::new("modprobe").args(["-r", module]).status() {
                log::debug!("Failed to run modprobe -r {}: {}", module, e);
            }
        }

        if let Some(pci_addr) = get_nvidia_gpu_pci_addr() {
            if let Err(e) = unbind_gpu(&pci_addr) {
                error!("Failed to unbind GPU: {}", e);
            }
            if let Err(e) = remove_gpu(&pci_addr) {
                error!("Failed to remove GPU: {}", e);
            }
        }

        guard
    } else {
        DmGuard { name: None }
    };

    let is_debug = log::log_enabled!(log::Level::Debug);
    let mut dis_cmd = Command::new("systemctl");
    dis_cmd.args(["disable", "nvidia-persistenced.service"]);
    if !is_debug {
        dis_cmd.stdout(std::process::Stdio::null());
        dis_cmd.stderr(std::process::Stdio::null());
    }
    match dis_cmd.status() {
        Ok(s) if s.success() => println!("Successfully disabled nvidia-persistenced.service"),
        Ok(s) => error!(
            "nvidia-persistenced.service disable failed with exit code: {}",
            s
        ),
        Err(e) => error!("Failed to run systemctl: {}", e),
    }

    if no_reboot {
        println!("Operation completed successfully");
    } else {
        rebuild_initramfs();
        println!("Operation completed successfully");
        println!("Please reboot your computer for changes to take effect!");
    }
}

fn switch_hybrid(rtd3: Option<u8>, use_nvidia_current: bool, no_reboot: bool) {
    assert_root();
    let _lock = match LockGuard::acquire() {
        Ok(l) => l,
        Err(e) => {
            error!("{}", e);
            return;
        }
    };
    println!("Switching to hybrid mode");
    println!(
        "Enable PCI-Express Runtime D3 (RTD3) Power Management: {}",
        rtd3.is_some()
    );

    if no_reboot && let Err(e) = rescan_pci() {
        error!("Failed to rescan PCI bus: {}", e);
    }

    if !Path::new(CACHE_FILE_PATH).exists() {
        create_cache_file();
    }

    cleanup();

    let _guard = if no_reboot {
        let guard = match prepare_no_reboot(None) {
            Ok(g) => g,
            Err(e) => {
                error!("{}", e);
                return;
            }
        };

        if let Err(e) = kill_gpu_processes() {
            error!("Failed to kill GPU processes: {}", e);
        }

        guard
    } else {
        DmGuard { name: None }
    };

    // persistenced enable
    let is_debug = log::log_enabled!(log::Level::Debug);
    let mut enable_cmd = Command::new("systemctl");
    enable_cmd.args(["enable", "nvidia-persistenced.service"]);
    if !is_debug {
        enable_cmd.stdout(std::process::Stdio::null());
        enable_cmd.stderr(std::process::Stdio::null());
    }
    match enable_cmd.status() {
        Ok(s) if s.success() => println!("Successfully enabled nvidia-persistenced.service"),
        Ok(s) => error!(
            "nvidia-persistenced.service enable failed with exit code: {}",
            s
        ),
        Err(e) => error!("Failed to run systemctl: {}", e),
    }

    if let Some(val) = rtd3 {
        let modeset_content = if use_nvidia_current {
            format!(
                "# Automatically generated by lapctl\n\noptions nvidia-current-drm modeset=1\noptions nvidia-current \"NVreg_DynamicPowerManagement=0x0{}\"\noptions nvidia-current NVreg_UsePageAttributeTable=1 NVreg_InitializeSystemMemoryAllocations=0\n",
                val
            )
        } else {
            format!(
                "# Automatically generated by lapctl\n\noptions nvidia-drm modeset=1\noptions nvidia \"NVreg_DynamicPowerManagement=0x0{}\"\noptions nvidia NVreg_UsePageAttributeTable=1 NVreg_InitializeSystemMemoryAllocations=0\n",
                val
            )
        };
        create_file(MODESET_PATH, &modeset_content, false);
        create_file(UDEV_PM_PATH, UDEV_PM_CONTENT, false);
    } else {
        let modeset_content = if use_nvidia_current {
            "# Automatically generated by lapctl\n\noptions nvidia-current-drm modeset=1\noptions nvidia-current NVreg_UsePageAttributeTable=1 NVreg_InitializeSystemMemoryAllocations=0\n".to_string()
        } else {
            "# Automatically generated by lapctl\n\noptions nvidia-drm modeset=1\noptions nvidia NVreg_UsePageAttributeTable=1 NVreg_InitializeSystemMemoryAllocations=0\n".to_string()
        };
        create_file(MODESET_PATH, &modeset_content, false);
    }

    if no_reboot {
        println!("Operation completed successfully");
    } else {
        rebuild_initramfs();
        println!("Operation completed successfully");
        println!("Please reboot your computer for changes to take effect!");
    }
}

fn switch_nvidia(
    dm: Option<String>,
    force_comp: bool,
    coolbits: Option<u32>,
    use_nvidia_current: bool,
    wayland: bool,
    no_reboot: bool,
) {
    assert_root();
    let _lock = match LockGuard::acquire() {
        Ok(l) => l,
        Err(e) => {
            error!("{}", e);
            return;
        }
    };

    let wayland = wayland || is_wayland_session();

    println!("Switching to nvidia mode");
    println!("Enable ForceCompositionPipeline: {}", force_comp);
    println!("Enable Coolbits: {}", coolbits.is_some());

    if no_reboot && let Err(e) = rescan_pci() {
        error!("Failed to rescan PCI bus: {}", e);
    }

    if !Path::new(CACHE_FILE_PATH).exists() {
        create_cache_file();
    }

    let cache = read_cache_file();
    let pci_bus = match cache {
        Ok(c) => c.nvidia_gpu_pci_bus,
        Err(_) => get_nvidia_gpu_pci_bus(),
    };

    cleanup();

    if use_nvidia_current {
        create_file(
            MODESET_PATH,
            "# Automatically generated by lapctl\n\noptions nvidia-current-drm modeset=1\noptions nvidia-current NVreg_UsePageAttributeTable=1 NVreg_InitializeSystemMemoryAllocations=0\n",
            false,
        );
    } else {
        create_file(
            MODESET_PATH,
            "# Automatically generated by lapctl\n\noptions nvidia-drm modeset=1\noptions nvidia NVreg_UsePageAttributeTable=1 NVreg_InitializeSystemMemoryAllocations=0\n",
            false,
        );
    }

    if wayland {
        create_file(
            "/etc/lapctl-wayland-nvidia",
            "# Wayland mode marker for lapctl\n",
            false,
        );
    } else {
        let igpu_vendor = get_igpu_vendor();
        if let Some(ref vendor) = igpu_vendor {
            if vendor == "intel" {
                create_file(XORG_PATH, &xorg_intel(&pci_bus), false);
            } else if vendor == "amd" {
                create_file(XORG_PATH, &xorg_amd(&pci_bus), false);
            }
        } else {
            create_file(XORG_PATH, &xorg_modesetting(&pci_bus), false);
        }

        let mut extra_xorg = EXTRA_XORG_CONTENT.to_string();
        if force_comp {
            extra_xorg.push_str("    Option \"ForceCompositionPipeline\" \"true\"\n");
        }
        if let Some(cb) = coolbits {
            extra_xorg.push_str(&format!("    Option \"Coolbits\" \"{}\"\n", cb));
        }
        if force_comp || coolbits.is_some() {
            extra_xorg.push_str("EndSection\n");
            create_file(EXTRA_XORG_PATH, &extra_xorg, false);
        }

        let display_manager = dm.clone().or_else(get_display_manager);

        if let Some(cdm) = display_manager {
            if cdm == "sddm" {
                if Path::new(SDDM_XSETUP_PATH).exists() {
                    info!("Creating Xsetup backup");
                    if let Ok(content) = fs::read_to_string(SDDM_XSETUP_PATH) {
                        create_file(&format!("{}.bak", SDDM_XSETUP_PATH), &content, false);
                    }
                }
                create_file(
                    SDDM_XSETUP_PATH,
                    &generate_xrandr_script(igpu_vendor.as_ref()),
                    true,
                );
            } else if cdm == "lightdm" {
                create_file(
                    LIGHTDM_SCRIPT_PATH,
                    &generate_xrandr_script(igpu_vendor.as_ref()),
                    true,
                );
                create_file(LIGHTDM_CONFIG_PATH, LIGHTDM_CONFIG_CONTENT, false);
            }
        }
    }

    let _guard = if no_reboot {
        let guard = match prepare_no_reboot(dm.clone()) {
            Ok(g) => g,
            Err(e) => {
                error!("{}", e);
                return;
            }
        };

        if let Err(e) = kill_gpu_processes() {
            error!("Failed to kill GPU processes: {}", e);
        }

        guard
    } else {
        DmGuard { name: None }
    };

    // persistenced enable
    let is_debug = log::log_enabled!(log::Level::Debug);
    let mut enable_cmd = Command::new("systemctl");
    enable_cmd.args(["enable", "nvidia-persistenced.service"]);
    if !is_debug {
        enable_cmd.stdout(std::process::Stdio::null());
        enable_cmd.stderr(std::process::Stdio::null());
    }
    match enable_cmd.status() {
        Ok(s) if s.success() => println!("Successfully enabled nvidia-persistenced.service"),
        Ok(s) => error!(
            "nvidia-persistenced.service enable failed with exit code: {}",
            s
        ),
        Err(e) => error!("Failed to run systemctl: {}", e),
    }

    if no_reboot {
        println!("Operation completed successfully");
    } else {
        rebuild_initramfs();
        println!("Operation completed successfully");
        println!("Please reboot your computer for changes to take effect!");
    }
}

fn run_on_dgpu(command: &[String]) {
    if command.is_empty() {
        error!("No command specified.");
        return;
    }

    if get_current_mode() != "hybrid" {
        error!("GPU run is only supported in Hybrid mode.");
        return;
    }

    let env_vars = vec![
        ("__NV_PRIME_RENDER_OFFLOAD", "1".to_string()),
        ("__GLX_VENDOR_LIBRARY_NAME", "nvidia".to_string()),
        ("__VK_LAYER_NV_optimus", "NVIDIA_only".to_string()),
    ];

    let mut cmd = Command::new(&command[0]);
    if command.len() > 1 {
        cmd.args(&command[1..]);
    }

    for (key, value) in &env_vars {
        cmd.env(key, value);
    }

    info!("Running '{}' on discrete GPU", command.join(" "));

    match cmd.status() {
        Ok(status) => {
            if !status.success() {
                error!("Command exited with status: {}", status);
            }
        }
        Err(e) => {
            error!("Failed to execute command: {}", e);
        }
    }
}

fn show_gpu_power_state() {
    println!("--- GPU Power State ---");

    // Read PCI power state from sysfs
    let pci_base = Path::new("/sys/bus/pci/devices");
    if let Ok(entries) = fs::read_dir(pci_base) {
        for entry in entries.flatten() {
            let vendor_path = entry.path().join("vendor");
            let class_path = entry.path().join("class");

            let Ok(vendor) = fs::read_to_string(&vendor_path) else {
                continue;
            };
            let Ok(class) = fs::read_to_string(&class_path) else {
                continue;
            };

            // NVIDIA vendor = 0x10de, VGA class starts with 0x0300
            if vendor.trim() == "0x10de" && class.trim().starts_with("0x0300") {
                let pci_addr = entry.file_name().to_string_lossy().to_string();
                println!("  Device: NVIDIA ({})", pci_addr);

                // Power state
                let runtime_status = entry.path().join("power/runtime_status");
                if let Ok(status) = fs::read_to_string(&runtime_status) {
                    println!("  Runtime Status: {}", status.trim());
                }

                // Power control
                let power_control = entry.path().join("power/control");
                if let Ok(control) = fs::read_to_string(&power_control) {
                    println!("  Power Control: {}", control.trim());
                }

                // Current power state (D0 = fully on, D3 = off)
                let current_state = entry.path().join("power_state");
                if let Ok(state) = fs::read_to_string(&current_state) {
                    println!("  PCI Power State: {}", state.trim());
                }

                // Max power state
                let max_state = entry.path().join("max_power_state");
                if let Ok(state) = fs::read_to_string(&max_state) {
                    println!("  Max Power State: {}", state.trim());
                }
            }
        }
    }

    // NVIDIA power info via nvidia-smi
    match std::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=power.draw,power.limit,power.default_limit,power.max_limit,clocks.current.graphics,clocks.max.graphics",
            "--format=csv,noheader",
        ])
        .output()
    {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split(", ").collect();
                if parts.len() >= 5 {
                    println!();
                    println!("  Power Draw: {}", parts[0].trim());
                    println!("  Power Limit: {}", parts[1].trim());
                    println!("  Default Limit: {}", parts[2].trim());
                    println!("  Max Limit: {}", parts[3].trim());
                    println!("  Current Clock: {}", parts[4].trim());
                    if parts.len() > 5 {
                        println!("  Max Clock: {}", parts[5].trim());
                    }
                }
            }
        }
        _ => {
            log::debug!("nvidia-smi not available or GPU not powered on");
            println!("  (nvidia-smi not available - GPU may be powered off)");
        }
    }
}

pub fn execute(cmd: &GpuCommands) {
    let cmd = match cmd {
        GpuCommands::Nvidia {
            wayland: false,
            dm,
            force_comp,
            coolbits,
            use_nvidia_current,
            no_reboot,
        } if is_wayland_session() => {
            println!("Auto-detected Wayland session");
            GpuCommands::Nvidia {
                wayland: true,
                dm: dm.clone(),
                force_comp: *force_comp,
                coolbits: *coolbits,
                use_nvidia_current: *use_nvidia_current,
                no_reboot: *no_reboot,
            }
        }
        _ => cmd.clone(),
    };

    match &cmd {
        GpuCommands::Integrated { .. } => println!("Switching to Integrated GPU mode..."),
        GpuCommands::Hybrid { .. } => println!("Switching to Hybrid GPU mode..."),
        GpuCommands::Nvidia { .. } => println!("Switching to NVIDIA-only GPU mode..."),
        _ => {}
    }

    if try_call_daemon(&cmd) {
        return;
    }

    execute_local(&cmd);
}

pub fn execute_local(cmd: &GpuCommands) {
    match cmd {
        GpuCommands::Query => {
            let mode = get_current_mode();
            println!("{}", mode);
        }
        GpuCommands::CacheCreate => {
            assert_root();
            create_cache_file();
        }
        GpuCommands::CacheDelete => {
            assert_root();
            if Path::new(CACHE_FILE_PATH).exists() {
                if let Err(e) = fs::remove_file(CACHE_FILE_PATH) {
                    error!("Failed to remove cache file: {}", e);
                }
                if let Some(parent) = Path::new(CACHE_FILE_PATH).parent()
                    && let Err(e) = fs::remove_dir(parent)
                {
                    log::debug!("Failed to remove cache directory: {}", e);
                }
            }
        }
        GpuCommands::CacheQuery => {
            if let Ok(content) = fs::read_to_string(CACHE_FILE_PATH) {
                println!("{}", content);
            } else {
                println!("ERROR: Could not read {}", CACHE_FILE_PATH);
            }
        }
        GpuCommands::Reset => {
            assert_root();

            if get_current_mode() == "hybrid" && !Path::new(CACHE_FILE_PATH).exists() {
                create_cache_file();
            }

            cleanup();
            if Path::new(CACHE_FILE_PATH).exists() {
                if let Err(e) = fs::remove_file(CACHE_FILE_PATH) {
                    error!("Failed to remove cache file: {}", e);
                }
                if let Some(parent) = Path::new(CACHE_FILE_PATH).parent()
                    && let Err(e) = fs::remove_dir(parent)
                {
                    log::debug!("Failed to remove cache directory: {}", e);
                }
            }
            rebuild_initramfs();
            println!("Operation completed successfully");
        }
        GpuCommands::ResetSddm => {
            assert_root();
            create_file(SDDM_XSETUP_PATH, SDDM_XSETUP_CONTENT, true);
            println!("Operation completed successfully");
        }
        GpuCommands::Integrated { no_reboot } => switch_integrated(*no_reboot),
        GpuCommands::Hybrid {
            rtd3,
            use_nvidia_current,
            no_reboot,
        } => switch_hybrid(*rtd3, *use_nvidia_current, *no_reboot),
        GpuCommands::Nvidia {
            dm,
            force_comp,
            coolbits,
            use_nvidia_current,
            wayland,
            no_reboot,
        } => switch_nvidia(
            dm.clone(),
            *force_comp,
            *coolbits,
            *use_nvidia_current,
            *wayland,
            *no_reboot,
        ),
        GpuCommands::Run { command } => run_on_dgpu(command),
        GpuCommands::Power => show_gpu_power_state(),
    }
}
