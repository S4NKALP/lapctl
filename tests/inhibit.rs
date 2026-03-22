use clap::Parser;
use lapctl::cli::{Cli, Commands};

#[test]
fn test_inhibit_cli_parsing() {
    let args = vec!["lapctl", "inhibit", "sleep", "10", "--why", "testing"];
    let cli = Cli::parse_from(args);

    if let Commands::Inhibit {
        command,
        why,
        who,
        daemon,
        stop: _,
    } = cli.command
    {
        assert_eq!(command, vec!["sleep".to_string(), "10".to_string()]);
        assert_eq!(why, "testing");
        assert_eq!(who, "lapctl"); // default value
        assert!(!daemon);
    } else {
        panic!("Command should be Inhibit");
    }
}

#[test]
fn test_inhibit_daemon_parsing() {
    let args = vec!["lapctl", "inhibit", "--daemon"];
    let cli = Cli::parse_from(args);

    if let Commands::Inhibit {
        command,
        why,
        who: _,
        daemon,
        stop: _,
    } = cli.command
    {
        assert!(command.is_empty());
        assert_eq!(why, "lapctl inhibiting sleep"); // default value
        assert!(daemon);
    } else {
        panic!("Command should be Inhibit");
    }
}
