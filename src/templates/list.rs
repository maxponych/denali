use colored::Colorize;

use crate::utils::{Errors, context::AppContext};

pub fn tmpl_list(ctx: &AppContext) -> Result<(), Errors> {
    let manifest = ctx.load_main_manifest()?;

    let mut names: Vec<&String> = manifest.templates.keys().collect();
    names.sort();

    println!("Templates");

    let total = names.len();
    for (i, name) in names.iter().enumerate() {
        let is_last = i + 1 == total;
        let branch = if is_last { "└─" } else { "├─" };
        println!("{} {}", branch, name.yellow());
    }

    Ok(())
}
