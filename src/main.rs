mod commands;
use commands::{add, checkout, init};
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
    Init {
        path: Option<PathBuf>,
    },
    Add {
        path: PathBuf,
    },
    Checkout {
        branch: String,
        dest: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { path } => {
            init(path.as_deref()).unwrap();
        }
        Commands::Add { path } => {
            add(&path).unwrap();
        }
        Commands::Checkout { branch, dest } => {
            checkout(branch, dest.as_deref()).unwrap();
        }
    }
}
