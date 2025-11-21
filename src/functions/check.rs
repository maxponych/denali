use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
};

use chrono::Utc;
use dialoguer::Confirm;
use uuid::Uuid;

use crate::utils::{
    CellConfig, CellRef, DenaliToml, Errors, MainManifest, ProjectManifest, ProjectRef,
    context::AppContext,
};

pub fn check(ctx: &AppContext, path: Option<&Path>) -> Result<(), Errors> {
    let root = match path {
        Some(p) => env::current_dir()?.join(p).canonicalize()?,
        None => env::current_dir()?,
    };

    ctx.make_root_dir()?;

    let config = read_config(&root)?;
    let manifest = load_manifest(&ctx.main_manifest_path())?;
    if !manifest.projects.contains_key(&config.root.name) {
        let mut try_key: Option<String> = None;
        for (name, proj_ref) in &manifest.projects {
            if proj_ref.path == root.to_string_lossy().to_string() {
                try_key = Some(name.to_string());
                break;
            }
        }

        if let Some(key) = try_key {
            update_proj_name_in_main(&ctx.main_manifest_path(), &key, &config.root.name)?;
        } else {
            create_proj(ctx, root, &config)?;
            return Ok(());
        }
    }

    let mut new_manifest = load_manifest(&ctx.main_manifest_path())?;

    let mut project = new_manifest
        .projects
        .get_mut(&config.root.name)
        .ok_or(Errors::InternalError)?;

    check_updates(ctx, &mut project, &config, &root, &config.root.name)?;
    println!("Everything seems good!");

    Ok(())
}

fn maybe_delete(
    ctx: &AppContext,
    name: &str,
    project_name: String,
    project_manifest: &ProjectManifest,
) -> Result<(), Errors> {
    let confirmed = Confirm::new()
        .with_prompt(format!(
            "The cell \"{}\" was removed from config. Do you wish to delete?",
            name
        ))
        .default(true)
        .wait_for_newline(false)
        .show_default(true)
        .interact()?;

    if confirmed {
        delete_cell(ctx, name.to_string(), project_name)?;
    } else {
        let cell_ref = project_manifest
            .cells
            .get(name)
            .ok_or(Errors::InternalError)?;
        let cell_conf = CellConfig {
            description: cell_ref.description.clone(),
            path: cell_ref.path.clone(),
            ignore: Vec::new(),
            lock: String::new(),
            snapshot_before: String::new(),
            snapshot_after: String::new(),
        };
        update_project_config(
            &Path::new(&project_manifest.source),
            name.to_string(),
            cell_conf,
        )?;
    }

    Ok(())
}

fn update_project_config(path: &Path, name: String, cell: CellConfig) -> Result<(), Errors> {
    let file_path = path.join(".denali.toml");
    let data = fs::read_to_string(&file_path)?;
    let mut config: DenaliToml = toml::from_str(&data)?;
    config.cells.insert(name, cell);
    let content = toml::to_string_pretty(&config)?;
    fs::write(file_path, content)?;
    Ok(())
}

fn delete_cell(ctx: &AppContext, cell: String, project_name: String) -> Result<(), Errors> {
    let mut manifest = load_manifest(&ctx.main_manifest_path())?;
    let proj_ref = manifest
        .projects
        .get_mut(&project_name)
        .ok_or(Errors::InternalError)?;
    let uuid = proj_ref.manifest.clone();
    let mut project_manifest = load_project_manifest(&ctx.project_manifest_path(uuid.clone()))?;
    proj_ref.cells.retain(|n| n != &cell);
    let path = ctx.project_manifest_path(uuid);
    project_manifest.cells.remove(&cell);
    let project_manifest_data = serde_json::to_vec_pretty(&project_manifest)?;
    fs::write(path, &project_manifest_data)?;
    let manifest_data = serde_json::to_vec_pretty(&manifest)?;
    fs::write(ctx.main_manifest_path(), &manifest_data)?;
    Ok(())
}

fn check_updates(
    ctx: &AppContext,
    project: &mut ProjectRef,
    config: &DenaliToml,
    path: &Path,
    project_name: &str,
) -> Result<(), Errors> {
    let project_conf = load_project_manifest(&ctx.project_manifest_path(project.manifest.clone()))?;
    if project.path != path.to_string_lossy().to_string() {
        let confirmed = Confirm::new()
            .with_prompt(format!(
                "The project \"{}\" had changed path from \"{}\" to \"{}\". Do you wish to change?",
                project_name,
                &project_conf.source,
                path.display()
            ))
            .default(true)
            .wait_for_newline(false)
            .show_default(true)
            .interact()?;

        if confirmed {
            change_project_path(
                ctx,
                &ctx.project_manifest_path(project.manifest.clone()),
                path,
                project,
                &config.root.name,
            )?;
        } else {
            return Err(Errors::Stopped);
        }
    }

    if project_conf.description != config.root.description {
        let confirmed = Confirm::new()
            .with_prompt(format!(
                "The project \"{}\" had changed description. Do you with to change?",
                project_name
            ))
            .default(true)
            .wait_for_newline(false)
            .show_default(true)
            .interact()?;

        if confirmed {
            change_project_description(
                &ctx.project_manifest_path(project.manifest.clone()),
                &config.root.description,
            )?;
        } else {
            return Err(Errors::Stopped);
        }
    }

    for (cell, cell_ref) in &config.cells {
        check_cell(ctx, &cell, cell_ref, &config.root.name, &project.manifest)?;
    }

    let new_manifest = load_project_manifest(&ctx.project_manifest_path(project.manifest.clone()))?;
    check_cells_delete(ctx, &new_manifest, config, &config.root.name)?;

    Ok(())
}

fn check_cell(
    ctx: &AppContext,
    name: &str,
    cell_conf: &CellConfig,
    proj_name: &str,
    uuid: &str,
) -> Result<(), Errors> {
    let manifest = load_project_manifest(&ctx.project_manifest_path(uuid.to_string()))?;
    if !manifest.cells.contains_key(name) {
        let mut try_key: Option<String> = None;
        for (name, cell_ref) in &manifest.cells {
            if cell_ref.path == cell_conf.path {
                try_key = Some(name.to_string());
                break;
            }
        }

        if let Some(key) = try_key {
            update_cell_name(ctx, &key, name, proj_name)?;
        } else {
            create_cell(ctx, name, &cell_conf, proj_name)?;
            return Ok(());
        }
    }

    let new_manifest = load_project_manifest(&ctx.project_manifest_path(uuid.to_string()))?;

    let cell_ref = new_manifest.cells.get(name).ok_or(Errors::InternalError)?;

    if cell_ref.description != cell_conf.description {
        let confirmed = Confirm::new()
            .with_prompt(format!(
                "The cell \"{}\" had changed description. Do you with to change?",
                name
            ))
            .default(true)
            .wait_for_newline(false)
            .show_default(true)
            .interact()?;

        if confirmed {
            change_cell_description(ctx, proj_name, name, cell_conf.description.clone())?;
        } else {
            return Err(Errors::Stopped);
        }
    }

    if cell_ref.path != cell_conf.path {
        let confirmed = Confirm::new()
            .with_prompt(format!(
                "The cell \"{}\" had changed path from \"{}\" to \"{}\". Do you wish to change?",
                name, cell_ref.path, cell_conf.path
            ))
            .default(true)
            .wait_for_newline(false)
            .show_default(true)
            .interact()?;

        if confirmed {
            change_cell_path(ctx, proj_name, name, cell_conf.path.clone())?;
        } else {
            return Err(Errors::Stopped);
        }
    }

    Ok(())
}

fn check_cells_delete(
    ctx: &AppContext,
    project: &ProjectManifest,
    config: &DenaliToml,
    proj_name: &str,
) -> Result<(), Errors> {
    for (name, _) in &project.cells {
        if let Some(_) = config.cells.get(name) {
            continue;
        } else {
            maybe_delete(ctx, name, proj_name.to_string(), project)?;
        }
    }
    Ok(())
}

fn change_cell_path(
    ctx: &AppContext,
    name: &str,
    cell_name: &str,
    path: String,
) -> Result<(), Errors> {
    let manifest = load_manifest(&ctx.main_manifest_path())?;
    let project_ref = manifest.projects.get(name).ok_or(Errors::InternalError)?;
    let uuid = project_ref.manifest.clone();
    let mut project_manifest = load_project_manifest(&ctx.project_manifest_path(uuid.clone()))?;
    project_manifest
        .cells
        .get_mut(cell_name)
        .ok_or(Errors::InternalError)?
        .path = path;
    save_project_manifest(&ctx.project_manifest_path(uuid), &project_manifest)?;
    Ok(())
}

fn change_cell_description(
    ctx: &AppContext,
    name: &str,
    cell_name: &str,
    description: String,
) -> Result<(), Errors> {
    let manifest = load_manifest(&ctx.main_manifest_path())?;
    let project_ref = manifest.projects.get(name).ok_or(Errors::InternalError)?;
    let uuid = project_ref.manifest.clone();
    let mut project_manifest = load_project_manifest(&ctx.project_manifest_path(uuid.clone()))?;
    project_manifest
        .cells
        .get_mut(cell_name)
        .ok_or(Errors::InternalError)?
        .description = description;
    save_project_manifest(&ctx.project_manifest_path(uuid), &project_manifest)?;
    Ok(())
}

fn update_cell_name(
    ctx: &AppContext,
    old_name: &str,
    new_name: &str,
    proj_name: &str,
) -> Result<(), Errors> {
    let confirmed = Confirm::new()
        .with_prompt(format!(
            "The cell \"{}\" changed name to \"{}\". Do you wish to rename it?",
            old_name, new_name
        ))
        .default(true)
        .wait_for_newline(false)
        .show_default(true)
        .interact()?;

    if confirmed {
        let mut manifest = load_manifest(&ctx.main_manifest_path())?;
        let project_ref = manifest
            .projects
            .get_mut(proj_name)
            .ok_or(Errors::InternalError)?;

        let idx = project_ref
            .cells
            .iter()
            .position(|c| c == old_name)
            .ok_or_else(|| Errors::InternalError)?;
        project_ref.cells.remove(idx);
        project_ref.cells.insert(idx, new_name.to_string());
        let uuid = project_ref.manifest.clone();
        let manifest_vec = serde_json::to_vec_pretty(&manifest)?;
        fs::write(ctx.main_manifest_path(), &manifest_vec)?;

        let mut project_manifest = load_project_manifest(&ctx.project_manifest_path(uuid.clone()))?;
        let cell_ref = project_manifest
            .cells
            .remove(old_name)
            .ok_or(Errors::InternalError)?;
        project_manifest
            .cells
            .insert(new_name.to_string(), cell_ref);
        let proj_manifest_data = serde_json::to_vec_pretty(&project_manifest)?;
        let file_path = ctx.project_manifest_path(uuid);
        fs::write(file_path, &proj_manifest_data)?;
    } else {
        return Err(Errors::InternalError);
    }
    Ok(())
}

fn create_cell(
    ctx: &AppContext,
    name: &str,
    cell: &CellConfig,
    proj_name: &str,
) -> Result<(), Errors> {
    let confirmed = Confirm::new()
        .with_prompt(format!(
            "The cell \"{}\" does not exist. Do you with to create it?",
            name
        ))
        .default(true)
        .wait_for_newline(false)
        .show_default(true)
        .interact()?;

    if confirmed {
        let mut manifest = load_manifest(&ctx.main_manifest_path())?;
        let project_ref = manifest
            .projects
            .get_mut(proj_name)
            .ok_or(Errors::InternalError)?;
        let uuid = project_ref.manifest.clone();
        project_ref.cells.push(name.to_string());
        let manifest_vec = serde_json::to_vec_pretty(&manifest)?;
        fs::write(ctx.main_manifest_path(), &manifest_vec)?;

        let cell_ref = CellRef {
            description: cell.description.clone(),
            path: cell.path.clone(),
            latest: String::new(),
            snapshots: HashMap::new(),
        };
        add_cell_to_project(&ctx.project_manifest_path(uuid), name, cell_ref)?;
    } else {
        return Err(Errors::Stopped);
    }
    Ok(())
}

fn change_project_description(path: &Path, description: &str) -> Result<(), Errors> {
    let manifest_data = fs::read(path)?;
    let mut manifest: ProjectManifest = serde_json::from_slice(&manifest_data)?;

    manifest.description = description.to_string();

    let json = serde_json::to_vec_pretty(&manifest)?;
    fs::write(path, json)?;
    Ok(())
}

fn change_project_path(
    ctx: &AppContext,
    manifest_path: &Path,
    path: &Path,
    project_ref: &mut ProjectRef,
    name: &str,
) -> Result<(), Errors> {
    let manifest_data = fs::read(manifest_path)?;
    let mut manifest: ProjectManifest = serde_json::from_slice(&manifest_data)?;

    manifest.source = path.to_string_lossy().to_string();
    project_ref.path = path.to_string_lossy().to_string();

    update_proj_in_main(&ctx.main_manifest_path(), name, project_ref)?;

    let json = serde_json::to_vec_pretty(&manifest)?;
    fs::write(manifest_path, json)?;
    Ok(())
}

fn read_config(path: &PathBuf) -> Result<DenaliToml, Errors> {
    let config_path = path.join(".denali.toml");
    let config_data = fs::read_to_string(config_path)?;
    let config: DenaliToml = toml::from_str(&config_data)?;
    Ok(config)
}

fn load_manifest(path: &Path) -> Result<MainManifest, Errors> {
    let manifest_data = fs::read(path)?;
    let manifest = serde_json::from_slice(&manifest_data)?;
    Ok(manifest)
}

fn create_proj(ctx: &AppContext, path: PathBuf, config: &DenaliToml) -> Result<(), Errors> {
    let confirmed = Confirm::new()
        .with_prompt("The project is not initialised, do you with to initialise it?")
        .default(true)
        .wait_for_newline(false)
        .show_default(true)
        .interact()?;
    if confirmed {
        let uuid = Uuid::new_v4();
        make_project_manifest(
            &ctx.project_manifest_path(uuid.to_string()),
            &path,
            config.root.description.clone(),
        )?;
        let mut new_project_ref = ProjectRef {
            path: path.to_string_lossy().to_string(),
            manifest: uuid.to_string(),
            latest: String::new(),
            cells: Vec::new(),
        };
        for (name, cell) in &config.cells {
            let cell_ref = CellRef {
                description: cell.description.clone(),
                path: cell.path.clone(),
                latest: String::new(),
                snapshots: HashMap::new(),
            };
            new_project_ref.cells.push(name.to_string());
            add_cell_to_project(&ctx.project_manifest_path(uuid.to_string()), name, cell_ref)?;
        }
        add_proj_to_main_manifest(
            &ctx.main_manifest_path(),
            &config.root.name,
            &new_project_ref,
        )?;
        Ok(())
    } else {
        return Err(Errors::Stopped);
    }
}

fn save_project_manifest(path: &Path, manifest: &ProjectManifest) -> Result<(), Errors> {
    let json = serde_json::to_vec_pretty(&manifest)?;
    fs::write(path, json)?;
    Ok(())
}

fn add_cell_to_project(file_path: &Path, name: &str, cell: CellRef) -> Result<(), Errors> {
    let manifest_data = fs::read(&file_path)?;
    let mut manifest: ProjectManifest = serde_json::from_slice(&manifest_data)?;
    if manifest.source == cell.path {
        return Err(Errors::ParentPath(cell.path));
    }
    manifest.cells.insert(name.to_string(), cell);
    let json = serde_json::to_vec_pretty(&manifest)?;
    fs::write(file_path, json)?;
    Ok(())
}

fn make_project_manifest(
    manifest_path: &Path,
    path: &Path,
    description: String,
) -> Result<ProjectManifest, Errors> {
    let project_manifest: ProjectManifest = ProjectManifest {
        source: path.to_string_lossy().to_string(),
        description: description,
        timestamp: Utc::now(),
        snapshots: HashMap::new(),
        cells: HashMap::new(),
    };

    let json = serde_json::to_vec_pretty(&project_manifest)?;
    fs::write(manifest_path, json)?;
    Ok(project_manifest)
}

fn load_project_manifest(path: &Path) -> Result<ProjectManifest, Errors> {
    let data = fs::read(path)?;
    let manifest = serde_json::from_slice(&data)?;
    Ok(manifest)
}

fn add_proj_to_main_manifest(
    path: &Path,
    name: &str,
    project_ref: &ProjectRef,
) -> Result<(), Errors> {
    let data = fs::read(path)?;
    let mut manifest: MainManifest = serde_json::from_slice(&data)?;
    manifest
        .projects
        .insert(name.to_string(), project_ref.clone());
    let data = serde_json::to_vec_pretty(&manifest)?;
    fs::write(path, data)?;
    Ok(())
}

fn update_proj_name_in_main(path: &Path, old_name: &str, new_name: &str) -> Result<(), Errors> {
    let confirmed = Confirm::new()
        .with_prompt(format!(
            "The project \"{}\" had changed name to \"{}\". Do you wish to change?",
            old_name, new_name
        ))
        .default(true)
        .wait_for_newline(false)
        .show_default(true)
        .interact()?;

    if confirmed {
        let data = fs::read(path)?;
        let mut manifest: MainManifest = serde_json::from_slice(&data)?;
        let proj_ref = manifest
            .projects
            .remove(old_name)
            .ok_or(Errors::InternalError)?;
        manifest.projects.insert(new_name.to_string(), proj_ref);
        let data = serde_json::to_vec_pretty(&manifest)?;
        fs::write(path, data)?;
    } else {
        return Err(Errors::Stopped);
    }

    Ok(())
}

fn update_proj_in_main(path: &Path, name: &str, project_ref: &ProjectRef) -> Result<(), Errors> {
    let data = fs::read(path)?;
    let mut manifest: MainManifest = serde_json::from_slice(&data)?;
    let entry = manifest
        .projects
        .get_mut(name)
        .ok_or(Errors::InternalError)?;

    *entry = project_ref.clone();
    let data = serde_json::to_vec_pretty(&manifest)?;
    fs::write(path, data)?;
    Ok(())
}
