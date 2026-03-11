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
    /// Hardware status
    Status,
}

#[derive(Subcommand, Debug)]
pub enum GpuCommands {
    /// Query the current graphics mode
    Query,
    /// Switch to integrated graphics
    Integrated,
    /// Switch to hybrid graphics
    Hybrid {
        /// Setup PCI-Express Runtime D3 (RTD3) Power Management on Hybrid mode (0, 1, 2, 3)
        #[arg(long)]
        rtd3: Option<u8>,
        /// Use nvidia-current instead of nvidia for kernel modules
        #[arg(long)]
        use_nvidia_current: bool,
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
}

#[derive(Subcommand, Debug)]
pub enum PowerCommands {
    /// Set power profile to performance
    Performance,
    /// Set power profile to balanced
    Balanced,
    /// Set power profile to battery saver
    BatterySave,
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
    Limit {
        percent: u8,
    },
    /// Show current battery status
    Status,
}
