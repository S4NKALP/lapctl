use clap::CommandFactory;
use lapctl::cli::Cli;

#[test]
fn verify_cli() {
    Cli::command().debug_assert();
}
