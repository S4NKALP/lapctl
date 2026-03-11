use clap::Parser;

mod cli;
mod commands;
mod hardware;
mod utils;

use cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    let level = if cli.verbose {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };

    env_logger::Builder::new()
        .filter_level(level)
        .format_timestamp(None)
        .format_module_path(false)
        .init();

    match &cli.command {
        Commands::Gpu { command } => commands::gpu::execute(command),
        Commands::Battery { command } => commands::battery::execute(command),
        Commands::Power { command } => commands::power::execute(command),
        Commands::Status => commands::status::execute(),
    }
}
