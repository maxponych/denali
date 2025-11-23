use crate::utils::TreeStruct;
use crate::utils::file_type::FileType;
use crate::utils::{Errors, TemplateRef, TmplToml, context::AppContext};
use dialoguer::Input;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::{collections::HashMap, env, fs, path::Path};

pub fn tmpl_apply(
    ctx: &AppContext,
    name: String,
    path: Option<&Path>,
    dry: bool,
    config: bool,
) -> Result<(), Errors> {
    let dir = match path {
        Some(p) => env::current_dir()?.join(p),
        None => env::current_dir()?,
    };

    let manifest = ctx.load_main_manifest()?;
    let template_ref = manifest
        .templates
        .get(&name)
        .ok_or(Errors::TemplateDoesntExist(name))?;

    restore(ctx, template_ref.tree.clone(), &dir)?;

    if config {
        let data = fs::read(template_ref.config.clone())?;
        fs::write(dir.join(".denali.tmpl.toml"), data)?;
    }

    if !dry {
        let placeholders = resolve_placeholders(&template_ref)?;
        execute_commands(&template_ref, &placeholders)?;
    }

    Ok(())
}

fn resolve_placeholders(template_ref: &TemplateRef) -> Result<HashMap<String, String>, Errors> {
    let config_vec = fs::read(Path::new(&template_ref.config))?;
    let config: TmplToml = toml::from_slice(&config_vec)?;

    let mut map: HashMap<String, String> = HashMap::new();

    for placeholder in config.placeholders {
        let val = Input::new()
            .with_prompt(format!("Enter a value for placeholder \"{}\"", placeholder))
            .interact_text()?;

        map.insert(format!("<{{{}}}>", placeholder), val);
    }

    Ok(map)
}

fn execute_commands(
    template_ref: &TemplateRef,
    placeholders: &HashMap<String, String>,
) -> Result<(), Errors> {
    let config_vec = fs::read(Path::new(&template_ref.config))?;
    let config: TmplToml = toml::from_slice(&config_vec)?;

    for cmd in config.commands {
        let mut resolved = cmd.clone();

        for (holder, val) in placeholders {
            resolved = resolved.replace(holder, val);
        }

        let status = Command::new("sh")
            .arg("-c")
            .arg(&resolved)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        if !status.success() {
            return Err(Errors::CommandFailed(resolved));
        }
    }

    Ok(())
}

fn restore_file(ctx: &AppContext, hash: String, dest: &Path, mode: [u8; 4]) -> Result<(), Errors> {
    let content = ctx.load_object(hash)?;

    if dest.exists() {
        fs::remove_file(dest)?;
    }

    fs::write(dest, content)?;
    let perms = u32::from_be_bytes(mode.clone()) & 0x0FFF;
    let mut permissions = fs::metadata(&dest)?.permissions();
    permissions.set_mode(perms);
    fs::set_permissions(&dest, permissions)?;
    Ok(())
}

fn parse_tree(tree: &Vec<u8>) -> Result<Vec<TreeStruct>, Errors> {
    let mut entries = Vec::new();

    let mut i = 0;
    while i < tree.len() {
        let mode_start = i;
        while tree[i] != b' ' {
            i += 1;
        }
        let mode: [u8; 4] = tree[mode_start..i].try_into()?;
        i += 1;

        let name_start = i;
        while tree[i] != 0 {
            i += 1;
        }
        let name = String::from_utf8_lossy(&tree[name_start..i]).to_string();
        i += 1;

        let hash: [u8; 32] = tree[i..i + 32].try_into()?;
        i += 32;

        entries.push(TreeStruct {
            mode,
            name: name,
            hash,
        });
    }

    Ok(entries)
}

fn restore(ctx: &AppContext, hash: String, dest: &Path) -> Result<(), Errors> {
    let tree = ctx.load_object(hash)?;

    let entries = parse_tree(&tree)?;

    for entry in entries {
        let target = dest.join(entry.name.clone());
        let mode = u32::from_be_bytes(entry.mode);
        let filetype = FileType::from_mode(mode);

        match filetype {
            FileType::Directory => {
                if !target.exists() {
                    fs::create_dir(&target)?;
                }
                let perms = mode & 0x0FFF;
                let mut permissions = fs::metadata(&target)?.permissions();
                permissions.set_mode(perms);
                fs::set_permissions(&target, permissions)?;

                restore(ctx, hex::encode(entry.hash), &target)?;
            }
            FileType::Symlink => {
                if target.exists() {
                    if target.is_dir() {
                        fs::remove_dir_all(&target)?;
                    } else {
                        fs::remove_file(&target)?;
                    }
                }
                let stored = ctx.load_object(hex::encode(entry.hash))?;
                let symlink_target = PathBuf::from(String::from_utf8_lossy(&stored).to_string());

                std::os::unix::fs::symlink(&symlink_target, &target)?;
            }
            FileType::Regular => {
                restore_file(ctx, hex::encode(entry.hash), &target, entry.mode)?;
            }
            _ => continue,
        }
    }

    Ok(())
}
