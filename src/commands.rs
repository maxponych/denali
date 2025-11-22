use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "denali", about = "Denali CLI tool")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(long, short, global = true)]
    pub root: Option<PathBuf>,
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
        #[arg(long, short)]
        wipe: bool,
    },
    Check {
        #[arg(long, short)]
        path: Option<PathBuf>,
    },
    Remove {
        project: String,
        name: Option<String>,
        #[arg(long, short)]
        all: bool,
    },
    Clean {
        #[arg(long, short)]
        dry: bool,
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
        project: String,
    },
}

#[derive(Subcommand)]
pub enum TmplCommand {
    Apply {
        name: String,
        #[arg(long, short)]
        path: Option<PathBuf>,
        #[arg(long, short)]
        dry: bool,
        #[arg(long = "with_config", short = 'c')]
        with_config: bool,
    },
    New {
        name: String,
        #[arg(long, short)]
        path: Option<PathBuf>,
        #[arg(long = "override", short = 'o')]
        over: bool,
    },
    List,
    Remove {
        name: String,
    },
}
