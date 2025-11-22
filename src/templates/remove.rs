use std::fs;

use crate::utils::{Errors, context::AppContext};

pub fn tmpl_remove(ctx: &AppContext, name: String) -> Result<(), Errors> {
    let mut manifest = ctx.load_main_manifest()?;

    if let Some(_) = manifest.templates.get(&name) {
        manifest.templates.remove(&name);
        ctx.write_main_manifest(&manifest)?;
        let tmpl_path = ctx.templates_path().join(format!("{}.toml", name));
        if tmpl_path.exists() {
            fs::remove_file(tmpl_path)?;
        }
    } else {
        return Err(Errors::TemplateDoesntExist(name));
    }

    Ok(())
}
