use crate::utils::{Errors, RemoteRef, context::AppContext};

pub fn remote_add(ctx: &AppContext, name: String, host: String) -> Result<(), Errors> {
    let mut manifest = ctx.load_main_manifest()?;
    let (host, path) = parse_remote_path(&host);
    let remote = RemoteRef { host, path };
    manifest.remotes.insert(name, remote);
    ctx.write_main_manifest(&manifest)?;
    Ok(())
}

fn parse_remote_path(input: &str) -> (String, String) {
    match input.split_once(':') {
        Some((host, path)) => (host.to_string(), path.to_string()),
        None => (input.to_string(), ".".to_string()),
    }
}
