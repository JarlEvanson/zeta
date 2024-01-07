use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(author)]
#[command(about = "An integrated utility script meant to simplify testing and updates to the zeta project and its binaries", long_about = None)]
pub struct App {
    #[command(subcommand)]
    sub_command: SubCommand,
}

impl App {
    pub fn execute(&self) {
        match &self.sub_command {
            SubCommand::UpdateChecksum {
                config_path,
                bootloader_path,
            } => {
                let config_file = std::fs::read(config_path).unwrap();

                let mut executable = std::fs::File::options()
                    .write(true)
                    .read(true)
                    .open(bootloader_path)
                    .unwrap();

                crate::config_checksum::update_config_checksum(
                    &mut executable,
                    digest::sha512::bytes::Sha512::hash(&config_file),
                )
                .unwrap();

                println!("Checksum updated");
            }
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum SubCommand {
    #[command(arg_required_else_help = true)]
    UpdateChecksum {
        config_path: String,
        bootloader_path: String,
    },
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum Profile {
    Debug,
    Release,
    Custom,
}
