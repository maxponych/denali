use crate::utils::{Errors, TemplateRef, TmplToml, context::AppContext};
use dialoguer::Input;
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

struct TreeStruct {
    mode: String,
    name: String,
    hash: [u8; 32],
}

fn restore_file(ctx: &AppContext, hash: String, dest: &Path) -> Result<(), Errors> {
    let content = ctx.load_object(hash)?;

    if dest.exists() {
        fs::remove_file(dest)?;
    }

    fs::write(dest, content)?;
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
        let mode = String::from_utf8_lossy(&tree[mode_start..i]).to_string();
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
            mode: mode,
            name: name,
            hash: hash,
        });
    }

    Ok(entries)
}

fn restore(ctx: &AppContext, hash: String, dest: &Path) -> Result<(), Errors> {
    let tree = ctx.load_object(hash)?;

    let entries = parse_tree(&tree)?;

    for entry in entries {
        let target = dest.join(entry.name.clone());

        if entry.mode == "10" {
            if !target.exists() {
                fs::create_dir(&target)?;
            }
            restore(ctx, hex::encode(entry.hash), &target)?;
        } else {
            restore_file(ctx, hex::encode(entry.hash), &target)?;
        }
    }

    Ok(())
}
