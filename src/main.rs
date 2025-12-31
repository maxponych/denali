mod commands;
mod functions;
mod remote;
mod templates;
mod utils;

use clap::Parser;
use commands::{Cli, Commands, RemoteCommands, TmplCommand};
use utils::{context::AppContext, *};

use colored::*;
use functions::*;
use remote::*;
use templates::*;

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
            wipe,
        } => load(
            &ctx,
            project,
            name,
            path.as_deref(),
            before,
            after,
            with_config,
            wipe,
        )?,
        Commands::List { project } => list(&ctx, project)?,
        Commands::Copy { project, path } =>
        /* copy(&ctx, project, path.as_deref())?, */
        {
            ()
        }
        Commands::Check { path } => check(&ctx, path.as_deref())?,
        Commands::Remove { project, name, all } => remove(&ctx, project, name, all)?,
        Commands::Clean { dry } => clean(&ctx, dry)?,
        Commands::Tmpl { sub } => match sub {
            TmplCommand::New { name, path, over } => tmpl_new(&ctx, name, path.as_deref(), over)?,
            TmplCommand::Apply {
                name,
                path,
                dry,
                with_config,
            } => tmpl_apply(&ctx, name, path.as_deref(), dry, with_config)?,
            TmplCommand::List => tmpl_list(&ctx)?,
            TmplCommand::Remove { name } => tmpl_remove(&ctx, name)?,
        },
        Commands::Sync { project, remote } => remote_sync(&ctx, project, remote)?,
        Commands::Remote { sub } => match sub {
            RemoteCommands::Receive => remote_receive(&ctx)?,
            RemoteCommands::Send => remote_send(&ctx)?,
            RemoteCommands::Manifest { name } => remote_manifest(&ctx, name)?,
            RemoteCommands::Add { name, host } => remote_add(&ctx, name, host)?,
            RemoteCommands::Remove { name } => remote_remove(&ctx, name)?,
        },
    }
    Ok(())
}
