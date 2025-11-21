mod commands;
mod functions;
mod utils;

use clap::Parser;
use commands::{Cli, Commands, TmplCommand};
use utils::{context::AppContext, *};

use colored::*;
use functions::*;

fn main() {
    if let Err(e) = run() {
        eprintln!("{}", e.to_string().red().bold());
        std::process::exit(1);
    }
}

fn run() -> Result<(), Errors> {
    let cli = Cli::parse();
    let ctx = AppContext::new(cli.root)?;
    match cli.command {
        Commands::Init {
            name,
            path,
            description,
        } => init(&ctx, name, path.as_deref(), description.as_deref())?,
        Commands::Save {
            project,
            name,
            description,
        } => save(&ctx, project, name, description.as_deref())?,
        Commands::Load {
            project,
            name,
            path,
            before,
            after,
            with_config,
        } => load(
            &ctx,
            project,
            name,
            path.as_deref(),
            before,
            after,
            with_config,
        )?,
        Commands::List { project } => list(&ctx, project)?,
        Commands::Copy { project, path } => copy(&ctx, project, path.as_deref())?,
        Commands::Check { path } => check(&ctx, path.as_deref())?,
        Commands::Remove { project } => remove(&ctx, project)?,
        Commands::Clean { dry } => clean(&ctx, dry)?,
        Commands::Tmpl { sub } => match sub {
            TmplCommand::New { name, path } => None.unwrap(),
            TmplCommand::Apply { name, path } => None.unwrap(),
            TmplCommand::List => None.unwrap(),
        },
    }
    Ok(())
}
