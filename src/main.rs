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
        Commands::Display { command } => commands::display::execute(command),
        Commands::Status => commands::status::execute(),
        Commands::InstallRules => commands::install_rules::execute(),
        Commands::Touchpad { command } => commands::touchpad::execute(command),
        Commands::Inhibit {
            command,
            why,
            who,
            daemon,
            stop,
        } => commands::inhibit::execute(command, why, who, *daemon, *stop),
        Commands::Daemon => {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                if let Err(e) = lapctl::daemon::run().await {
                    eprintln!("Daemon error: {}", e);
                    std::process::exit(1);
                }
            });
        }
    }
}
