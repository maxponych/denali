mod commands;
mod functions;
mod objects;
mod utils;
use clap::Parser;
use commands::{Cli, Commands, TmplCommand};
use utils::*;

use colored::*;
use functions::init;
use functions::load;
use functions::save;

fn main() {
    if let Err(e) = run() {
        eprintln!("{}", e.to_string().red().bold());
        std::process::exit(1);
    }
}

fn run() -> Result<(), Errors> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init {
            name,
            path,
            description,
        } => init(name, path.as_deref(), description.as_deref())?,
        Commands::Save {
            project,
            name,
            description,
        } => save(project, name, description.as_deref())?,
        Commands::Load {
            project,
            name,
            path,
            before,
            after,
            with_config,
        } => load(project, name, path.as_deref(), before, after, with_config)?,
        Commands::Copy { project, path } => None.unwrap(),
        Commands::List { project } => None.unwrap(),
        Commands::Tmpl { sub } => match sub {
            TmplCommand::New { name, path } => None.unwrap(),
            TmplCommand::Apply { name, path } => None.unwrap(),
            TmplCommand::List => None.unwrap(),
        },
        Commands::Check { path } => None.unwrap(),
    }
    Ok(())
}
