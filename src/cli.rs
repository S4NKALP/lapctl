use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "lapctl",
    version,
    about = "CLI utility for managing laptop hardware features on Linux."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// GPU management commands
    Gpu {
        #[command(subcommand)]
        command: GpuCommands,
    },
    /// Battery management commands
    Battery {
        #[command(subcommand)]
        command: BatteryCommands,
    },
    /// Power management commands
    Power {
        #[command(subcommand)]
        command: PowerCommands,
    },
    /// Cooling and thermal management
    Cooling {
        #[command(subcommand)]
        command: CoolingCommands,
    },
    /// Display management commands
    Display {
        #[command(subcommand)]
        command: DisplayCommands,
    },
    /// Hardware status
    Status,
    /// Install udev rules for rootless operation
    InstallRules,
    /// Touchpad management
    Touchpad {
        #[command(subcommand)]
        command: TouchpadCommands,
    },
    /// Inhibit system sleep/suspend
    Inhibit {
        /// The command to run while inhibiting (optional)
        command: Vec<String>,
        /// Why the system is being inhibited
        #[arg(long, default_value = "lapctl inhibiting sleep")]
        why: String,
        /// Who is inhibiting the system
        #[arg(long, default_value = "lapctl")]
        who: String,
        /// Run the inhibitor in the background (daemon mode)
        #[arg(long)]
        daemon: bool,
        /// Stop any active persistent inhibition managed by the daemon
        #[arg(long)]
        stop: bool,
    },
    /// Start the lapctl background daemon (requires root)
    Daemon,
}

#[derive(Subcommand, Debug)]
pub enum GpuCommands {
    /// Query the current graphics mode
    Query,
    /// Switch to integrated graphics
    Integrated {
        /// Switch without rebooting (experimental)
        #[arg(long)]
        no_reboot: bool,
    },
    /// Switch to hybrid graphics
    Hybrid {
        /// Setup PCI-Express Runtime D3 (RTD3) Power Management on Hybrid mode (0, 1, 2, 3)
        #[arg(long)]
        rtd3: Option<u8>,
        /// Use nvidia-current instead of nvidia for kernel modules
        #[arg(long)]
        use_nvidia_current: bool,
        /// Switch without rebooting (experimental)
        #[arg(long)]
        no_reboot: bool,
    },
    /// Switch to NVIDIA graphics
    Nvidia {
        /// Manually specify your Display Manager (gdm, sddm, lightdm)
        #[arg(long)]
        dm: Option<String>,
        /// Enable ForceCompositionPipeline on Nvidia mode
        #[arg(long)]
        force_comp: bool,
        /// Enable Coolbits on Nvidia mode
        #[arg(long)]
        coolbits: Option<u32>,
        /// Use nvidia-current instead of nvidia for kernel modules
        #[arg(long)]
        use_nvidia_current: bool,
        /// Set up Nvidia mode for Wayland (skips Xorg configuration)
        #[arg(long)]
        wayland: bool,
        /// Switch without rebooting (experimental)
        #[arg(long)]
        no_reboot: bool,
    },
    /// Revert changes made by lapctl
    Reset,
    /// Restore default sddm Xsetup file
    ResetSddm,
    /// Create cache used by lapctl (hybrid mode only)
    CacheCreate,
    /// Delete cache created by lapctl
    CacheDelete,
    /// Show cache created by lapctl
    CacheQuery,
    /// Run an application on the discrete GPU (Requires Hybrid Mode)
    Run {
        /// The command and arguments to execute
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum PowerCommands {
    /// Set power profile to performance
    Performance,
    /// Set power profile to balanced
    Balanced,
    /// Set power profile to battery saver
    BatterySave,
    /// Set CPU TDP limit (in Watts)
    LimitTdp { watts: u32 },
}

#[derive(Subcommand, Debug)]
pub enum CoolingCommands {
    /// Set extreme performance fan/thermal mode
    Performance,
    /// Set balanced/intelligent cooling mode
    Balanced,
    /// Set quiet/battery saving fan mode
    Quiet,
}

#[derive(Subcommand, Debug)]
pub enum BatteryCommands {
    /// Set the battery charging limit (e.g., 80)
    Limit { percent: u8 },
    /// Show current battery status
    Status,
}

#[derive(Subcommand, Debug)]
pub enum TouchpadCommands {
    /// Enable the touchpad
    Enable,
    /// Disable the touchpad
    Disable,
}

#[derive(Subcommand, Debug)]
pub enum DisplayCommands {
    /// Show available and active refresh rates
    Rates,
    /// Set the display refresh rate
    SetRate {
        /// Target refresh rate in Hz
        rate: f32,
    },
}
