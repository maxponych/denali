use std::{
    env,
    fs::{self, File},
    io::Read,
    os::unix::fs::MetadataExt,
    path::Path,
};

use crate::utils::{Errors, TemplateRef, TreeStruct, context::AppContext};

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

fn build_tree(ctx: &AppContext, entries: Vec<TreeStruct>) -> Result<[u8; 32], Errors> {
    let mut content = Vec::new();

    for entry in entries {
        content.extend_from_slice(&entry.mode);
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
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            let name_os = path
                .file_name()
                .ok_or(Errors::DoesntExist(path.to_path_buf()))?;

            let meta = fs::symlink_metadata(path.clone())?;
            let mode = meta.mode().to_be_bytes();

            let hash = if meta.file_type().is_symlink() {
                let target = fs::read_link(&path)?;
                ctx.save_object(target.to_string_lossy().as_bytes().to_vec())?
            } else if meta.is_dir() {
                snapshot_dir(ctx, &path)?
            } else {
                hash_file(ctx, &path)?
            };

            if name_os != ".denali.tmpl.toml" {
                entries.push(TreeStruct {
                    mode,
                    name: name_os.to_string_lossy().to_string(),
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
            let meta = fs::symlink_metadata(path).unwrap();
            let mode = meta.mode().to_be_bytes();
            entries.push(TreeStruct {
                mode,
                name: name_os.to_string_lossy().to_string(),
                hash,
            });
        }
    }

    let hash = build_tree(ctx, entries)?;
    Ok(hash)
}
