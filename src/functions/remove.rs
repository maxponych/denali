use std::{fs, path::Path};

use crate::utils::{CellRef, DenaliToml, Errors, MainManifest, ProjectManifest, denali_root};

pub fn remove(project: String) -> Result<(), Errors> {
    let mut parts = project.split('@');
    let cell = parts.next().map(|s| s.to_string());
    let proj_name = parts.next().map(|s| s.to_string());

    let (cell, project_name) = match (cell, proj_name) {
        (Some(cell), Some(proj)) => (Some(cell), proj),
        (Some(proj), None) => (None, proj),
        _ => return Err(Errors::InvalidNameFormat(project)),
    };

    let manifest = load_manifest()?;

    if manifest.projects.get(&project_name).is_some() && cell.is_none() {
        delete_project(project_name)?;
        return Ok(());
    }

    delete_cell(
        cell.ok_or(Errors::InternalError)?,
        project_name.clone(),
        &Path::new(
            &manifest
                .projects
                .get(&project_name)
                .ok_or(Errors::InternalError)?
                .path,
        ),
    )?;

    Ok(())
}

fn load_config(path: &Path) -> Result<DenaliToml, Errors> {
    let file_path = path.join(".denali.toml");
    let data = fs::read_to_string(&file_path)?;
    let config: DenaliToml = toml::from_str(&data)?;
    Ok(config)
}

fn save_config(path: &Path, config: &DenaliToml) -> Result<(), Errors> {
    let file_path = path.join(".denali.toml");
    let data = toml::to_string_pretty(config)?;
    fs::write(file_path, data)?;
    Ok(())
}

fn delete_cell(cell: String, project_name: String, path: &Path) -> Result<(), Errors> {
    let mut config = load_config(path)?;
    let mut manifest = load_manifest()?;
    let proj_ref = manifest
        .projects
        .get_mut(&project_name)
        .ok_or(Errors::InternalError)?;
    let uuid = proj_ref.manifest.clone();
    let mut project_manifest = load_project_manifest(uuid.clone())?;
    proj_ref.cells.retain(|n| n != &cell);
    let project_path = denali_root()
        .join("snapshots")
        .join("projects")
        .join(format!("{}.json", uuid));
    project_manifest.cells.remove(&cell);
    let project_manifest_data = serde_json::to_vec_pretty(&project_manifest)?;
    fs::write(project_path, &project_manifest_data)?;
    let manifest_data = serde_json::to_vec_pretty(&manifest)?;
    fs::write(denali_root().join("manifest.json"), &manifest_data)?;
    config.cells.remove(&cell).ok_or(Errors::InternalError)?;
    save_config(path, &config)?;
    Ok(())
}

fn delete_project(project_name: String) -> Result<(), Errors> {
    let mut manifest = load_manifest()?;
    let proj_ref = manifest
        .projects
        .get_mut(&project_name)
        .ok_or(Errors::InternalError)?;
    let uuid = proj_ref.manifest.clone();
    let path = denali_root()
        .join("snapshots")
        .join("projects")
        .join(format!("{}.json", uuid));
    fs::remove_file(path)?;
    manifest
        .projects
        .remove(&project_name)
        .ok_or(Errors::InternalError)?;
    let manifest_data = serde_json::to_vec_pretty(&manifest)?;
    fs::write(denali_root().join("manifest.json"), &manifest_data)?;
    Ok(())
}

fn load_project_manifest(uuid: String) -> Result<ProjectManifest, Errors> {
    let path = denali_root()
        .join("snapshots")
        .join("projects")
        .join(format!("{}.json", uuid));
    let data = fs::read(path)?;
    let manifest = serde_json::from_slice(&data)?;
    Ok(manifest)
}
fn load_manifest() -> Result<MainManifest, Errors> {
    let path = denali_root().join("manifest.json");
    let manifest_data = fs::read(path)?;
    let manifest = serde_json::from_slice(&manifest_data)?;
    Ok(manifest)
}
