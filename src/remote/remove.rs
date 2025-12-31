use crate::utils::{Errors, context::AppContext};

pub fn remote_remove(ctx: &AppContext, name: String) -> Result<(), Errors> {
    let mut manifest = ctx.load_main_manifest()?;
    manifest.remotes.remove(&name);
    ctx.write_main_manifest(&manifest)?;
    Ok(())
}
