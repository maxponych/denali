use std::{
    env,
    ffi::OsString,
    fs::{self, File},
    io::Read,
    os::unix::ffi::OsStrExt,
    path::Path,
};

use crate::utils::{Errors, TemplateRef, context::AppContext};

pub fn tmpl_new(
    ctx: &AppContext,
    name: String,
    path: Option<&Path>,
    over: bool,
) -> Result<(), Errors> {
    ctx.make_root_dir()?;
    let mut manifest = ctx.load_main_manifest()?;

    if let Some(_) = manifest.templates.get(&name) {
        if !over {
            return Err(Errors::TemplateExists(name));
        }
    }

    let dir = match path {
        Some(p) => env::current_dir()?.join(p),
        None => env::current_dir()?,
    };

    if !dir.exists() {
        return Err(Errors::DoesntExist(dir));
    } else if !dir.is_dir() {
        return Err(Errors::NotADir(dir));
    }

    let mut template_data = Vec::new();
    if dir.join(".denali.tmpl.toml").exists() {
        template_data = fs::read(dir.join(".denali.tmpl.toml"))?;
    }

    let hash = snapshot_dir(ctx, &dir)?;
    let config_path = ctx.templates_path().join(format!("{}.toml", name));
    fs::write(&config_path, template_data)?;

    manifest.templates.insert(
        name,
        TemplateRef {
            tree: hex::encode(hash),
            config: config_path.to_string_lossy().to_string(),
        },
    );
    ctx.write_main_manifest(&manifest)?;
    Ok(())
}

struct TreeStruct {
    mode: String,
    name: OsString,
    hash: [u8; 32],
}

fn build_tree(ctx: &AppContext, entries: Vec<TreeStruct>) -> Result<[u8; 32], Errors> {
    let mut content = Vec::new();

    for entry in entries {
        content.extend_from_slice(entry.mode.as_bytes());
        content.push(b' ');
        content.extend_from_slice(entry.name.as_bytes());
        content.push(0);
        content.extend_from_slice(&entry.hash);
    }

    let hash = ctx.save_object(content)?;
    Ok(hash)
}

fn hash_file(ctx: &AppContext, path: &Path) -> Result<[u8; 32], Errors> {
    let mut file = File::open(path)?;
    let mut content = Vec::new();
    file.read_to_end(&mut content)?;

    let hash = ctx.save_object(content)?;
    Ok(hash)
}

fn snapshot_dir(ctx: &AppContext, path: &Path) -> Result<[u8; 32], Errors> {
    let mut entries: Vec<TreeStruct> = Vec::new();

    let mode_dir = "10";
    let mode_file = "20";

    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            let name_os = path
                .file_name()
                .ok_or(Errors::DoesntExist(path.to_path_buf()))?;

            let hash = if path.is_dir() {
                snapshot_dir(ctx, &path)?
            } else {
                hash_file(ctx, &path)?
            };

            if name_os != ".denali.tmpl.toml" {
                entries.push(TreeStruct {
                    mode: if path.is_dir() { mode_dir } else { mode_file }.to_string(),
                    name: name_os.to_os_string(),
                    hash,
                });
            }
        }
    } else {
        let name_os = path
            .file_name()
            .ok_or(Errors::DoesntExist(path.to_path_buf()))?;
        if name_os != ".denali.tmpl.toml" {
            let hash = hash_file(ctx, path)?;
            entries.push(TreeStruct {
                mode: mode_file.to_string(),
                name: name_os.to_os_string(),
                hash,
            });
        }
    }

    let hash = build_tree(ctx, entries)?;
    Ok(hash)
}
