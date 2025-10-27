mod commands;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "denali", about = "Denali CLI tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Init { path: Option<PathBuf> },
    Add { path: PathBuf },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { path } => {
            commands::init::init(path.as_deref()).unwrap();
        }
        Commands::Add { path } => {
            commands::add::add(&path).unwrap();
        }
    }
}
