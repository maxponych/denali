use crate::DenaliToml;
use crate::utils::context::AppContext;
use crate::utils::*;
use chrono::Utc;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{env, fs};
use uuid::Uuid;

fn make_project_manifest(
    manifest_path: PathBuf,
    path: &Path,
    description: String,
) -> Result<(), Errors> {
    let project_manifest: ProjectManifest = ProjectManifest {
        source: path.to_string_lossy().to_string(),
        description: description,
        timestamp: Utc::now(),
        snapshots: HashMap::new(),
        cells: HashMap::new(),
    };

    let json = serde_json::to_vec_pretty(&project_manifest)?;
    fs::write(manifest_path, json)?;
    Ok(())
}

fn make_config(path: &Path, config: DenaliToml) -> Result<(), Errors> {
    let config_path = path.join(".denali.toml");
    if config_path.exists() {
        return Err(Errors::ConfigExists(
            config_path.to_string_lossy().to_string(),
        ));
    }

    let file_data = toml::to_string_pretty(&config)?;
    fs::write(config_path, file_data)?;
    Ok(())
}

fn update_project_manifest_cell(path: PathBuf, name: String, cell: CellRef) -> Result<(), Errors> {
    let manifest_data = fs::read(&path)?;
    let mut manifest: ProjectManifest = serde_json::from_slice(&manifest_data)?;
    if manifest.source == cell.path {
        return Err(Errors::ParentPath(cell.path));
    }
    manifest.cells.insert(name, cell);
    let json = serde_json::to_vec_pretty(&manifest)?;
    fs::write(path, json)?;
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

pub fn init(
    ctx: &AppContext,
    name: String,
    path: Option<&Path>,
    description: Option<&str>,
) -> Result<(), Errors> {
    let dir = match path {
        Some(p) => env::current_dir()?.join(p),
        None => env::current_dir()?,
    };

    let desc = description.unwrap_or("");

    if !dir.exists() {
        return Err(Errors::DoesntExist(dir));
    } else if !dir.is_dir() {
        return Err(Errors::NotADir(dir));
    }

    ctx.make_root_dir()?;

    let manifest_path = ctx.main_manifest_path();
    let manifest_data = fs::read(&manifest_path)?;
    let mut manifest: MainManifest = serde_json::from_slice(&manifest_data)?;

    let mut parts = name.split('@');
    let cell = parts.next().map(|s| s.to_string());
    let project = parts.next().map(|s| s.to_string());

    let (cell, project) = match (cell, project) {
        (Some(cell), Some(project)) => (Some(cell), project),
        (Some(project), None) => (None, project),
        _ => return Err(Errors::InvalidNameFormat(name)),
    };

    if let Some(proj_ref) = manifest.projects.get(&project) {
        if cell.is_none() {
            return Err(Errors::SameName(project));
        } else if proj_ref.path == dir.to_string_lossy() {
            return Err(Errors::AlreadyInitialised);
        }
    }

    let uuid = Uuid::new_v4();

    let project_ref = ProjectRef {
        path: dir.to_string_lossy().to_string(),
        manifest: uuid.to_string(),
        latest: String::new(),
        cells: Vec::new(),
    };

    if cell.is_none() {
        let config_data = DenaliToml {
            root: ProjectConfig {
                name: project.clone(),
                description: desc.to_string(),
                ignore: Vec::new(),
                snapshot_before: String::new(),
                snapshot_after: String::new(),
            },
            cells: HashMap::new(),
        };
        make_config(&dir, config_data)?;
        make_project_manifest(
            ctx.project_manifest_path(uuid.to_string()),
            &dir,
            desc.to_string(),
        )?;
        manifest.projects.insert(project.clone(), project_ref);
    } else {
        let cell_name = cell.unwrap();
        let proj_ref = manifest
            .projects
            .get_mut(&project)
            .ok_or(Errors::ProjectNotFound(project.clone()))?;

        proj_ref.cells.push(cell_name.to_string());
        let new_cell = CellRef {
            description: desc.to_string(),
            path: dir.to_string_lossy().to_string(),
            latest: String::new(),
            snapshots: HashMap::new(),
        };
        let cell_conf = CellConfig {
            description: desc.to_string(),
            path: dir.to_string_lossy().to_string(),
            ignore: Vec::new(),
            lock: String::new(),
            snapshot_after: String::new(),
            snapshot_before: String::new(),
        };
        update_project_manifest_cell(
            ctx.project_manifest_path(proj_ref.manifest.clone()),
            cell_name.clone(),
            new_cell,
        )?;
        update_project_config(Path::new(&proj_ref.path), cell_name.clone(), cell_conf)?;
    }

    let json = serde_json::to_vec_pretty(&manifest)?;
    fs::write(&manifest_path, json)?;

    Ok(())
}
