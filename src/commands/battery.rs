use crate::cli::BatteryCommands;

pub fn execute(command: &BatteryCommands) {
    match command {
        BatteryCommands::Limit { percent } => {
            println!("Setting battery limit to {}%", percent);
        }
        BatteryCommands::Status => {
            println!("Battery status placeholder");
        }
    }
}
