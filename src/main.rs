use clap::Parser;

use lapctl::cli::{Cli, Commands};
use lapctl::commands;

fn main() {
    let cli = Cli::parse();

    let level = if cli.verbose {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };

    use std::io::Write;
    env_logger::Builder::new()
        .filter_level(level)
        .format(|buf, record| {
            // Write only [LEVEL] message
            writeln!(buf, "[{}] {}", record.level(), record.args())
        })
        .init();

    match &cli.command {
        Commands::Gpu { command } => commands::gpu::execute(command),
        Commands::Battery { command } => commands::battery::execute(command),
        Commands::Power { command } => commands::power::execute(command),
        Commands::Cooling { command } => commands::cooling::execute(command),
        Commands::Status => commands::status::execute(),
        Commands::InstallRules => commands::install_rules::execute(),
        Commands::Inhibit {
            command,
            why,
            who,
            daemon,
        } => commands::inhibit::execute(command, why, who, *daemon),
    }
}
