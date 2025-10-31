use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "denali", about = "Denali CLI tool")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Init {
        name: String,
        #[arg(long, short)]
        path: Option<PathBuf>,
        #[arg(long, short)]
        description: Option<String>,
    },
    Save {
        project: String,
        name: String,
        #[arg(long, short)]
        description: Option<String>,
    },
    Load {
        project: String,
        name: Option<String>,
        #[arg(long, short)]
        path: Option<PathBuf>,
        #[arg(long, short)]
        before: Option<String>,
        #[arg(long, short)]
        after: Option<String>,
        #[arg(long = "with_config", short = 'c')]
        with_config: bool,
    },
    Tmpl {
        #[command(subcommand)]
        sub: TmplCommand,
    },
    Copy {
        project: String,
        #[arg(long, short)]
        path: Option<PathBuf>,
    },
    List {
        project: Option<String>,
    },
    Check {
        #[arg(long, short)]
        path: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
pub enum TmplCommand {
    Apply {
        name: String,
        #[arg(long, short)]
        path: Option<PathBuf>,
    },
    New {
        name: String,
        #[arg(long, short)]
        path: Option<PathBuf>,
    },
    List,
}
