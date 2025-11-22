use std::{collections::HashMap, fs, path::Path};

use chrono::{DateTime, Utc};

use crate::utils::{DenaliToml, Errors, Snapshots, context::AppContext, parse_name};

pub fn remove(
    ctx: &AppContext,
    project: String,
    name: Option<String>,
    all: bool,
) -> Result<(), Errors> {
    let (project_name, cell) = parse_name(project)?;

    let manifest = ctx.load_main_manifest()?;

    if let Some(n) = name {
        if let Some(cell_name) = cell {
            delete_cell_snapshot(ctx, cell_name, project_name.clone(), n)?;
            return Ok(());
        } else {
            if !all {
                if manifest.projects.get(&project_name).is_some() && cell.is_none() {
                    delete_project_snapshot(ctx, project_name, n)?;
                    return Ok(());
                }
            } else {
                if let Some(project_ref) = manifest.projects.get(&project_name) {
                    if cell.is_none() {
                        delete_project_snapshot(ctx, project_name.clone(), n.clone())?;
                        for cell in project_ref.cells.clone().into_iter() {
                            delete_cell_snapshot(ctx, cell, project_name.clone(), n.clone())?;
                        }
                        return Ok(());
                    }
                }
            }
        }
        return Ok(());
    }

    if manifest.projects.get(&project_name).is_some() && cell.is_none() {
        delete_project(ctx, project_name)?;
        return Ok(());
    }

    delete_cell(
        ctx,
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

fn delete_cell_snapshot(
    ctx: &AppContext,
    cell: String,
    project_name: String,
    name: String,
) -> Result<(), Errors> {
    let manifest = ctx.load_main_manifest()?;

    let proj_ref = manifest
        .projects
        .get(&project_name)
        .ok_or(Errors::InternalError)?;

    let uuid = proj_ref.manifest.clone();

    let mut project_manifest = ctx.load_project_manifest(uuid.clone())?;

    let cell = project_manifest
        .cells
        .get_mut(&cell)
        .ok_or(Errors::InternalError)?;

    cell.snapshots.remove(&name);

    let snapshot = get_latest_snapshot(&cell.snapshots)?;
    cell.latest = snapshot;

    ctx.write_project_manifest(uuid, &project_manifest)?;

    Ok(())
}

fn delete_project_snapshot(
    ctx: &AppContext,
    project_name: String,
    name: String,
) -> Result<(), Errors> {
    let mut manifest = ctx.load_main_manifest()?;
    let proj_ref = manifest
        .projects
        .get_mut(&project_name)
        .ok_or(Errors::InternalError)?;
    let uuid = proj_ref.manifest.clone();
    let mut project_manifest = ctx.load_project_manifest(uuid.clone())?;
    project_manifest.snapshots.remove(&name);
    let snapshot = get_latest_snapshot(&project_manifest.snapshots)?;
    proj_ref.latest = snapshot;
    ctx.write_main_manifest(&manifest)?;
    ctx.write_project_manifest(uuid, &project_manifest)?;
    Ok(())
}

fn delete_cell(
    ctx: &AppContext,
    cell: String,
    project_name: String,
    path: &Path,
) -> Result<(), Errors> {
    let mut config = load_config(path)?;
    let mut manifest = ctx.load_main_manifest()?;
    let proj_ref = manifest
        .projects
        .get_mut(&project_name)
        .ok_or(Errors::InternalError)?;
    let uuid = proj_ref.manifest.clone();
    let mut project_manifest = ctx.load_project_manifest(uuid.clone())?;
    proj_ref.cells.retain(|n| n != &cell);
    project_manifest.cells.remove(&cell);
    ctx.write_project_manifest(uuid.clone(), &project_manifest)?;
    ctx.write_main_manifest(&manifest)?;
    config.cells.remove(&cell).ok_or(Errors::InternalError)?;
    save_config(path, &config)?;
    Ok(())
}

fn delete_project(ctx: &AppContext, project_name: String) -> Result<(), Errors> {
    let mut manifest = ctx.load_main_manifest()?;
    let proj_ref = manifest
        .projects
        .get_mut(&project_name)
        .ok_or(Errors::InternalError)?;
    let uuid = proj_ref.manifest.clone();
    let path = ctx.project_manifest_path(uuid);
    fs::remove_file(path)?;
    manifest
        .projects
        .remove(&project_name)
        .ok_or(Errors::InternalError)?;
    let manifest_data = serde_json::to_vec_pretty(&manifest)?;
    fs::write(ctx.main_manifest_path(), &manifest_data)?;
    Ok(())
}

fn get_latest_snapshot(snapshots: &HashMap<String, Snapshots>) -> Result<String, Errors> {
    let mut newest_timestamp: Option<DateTime<Utc>> = None;
    let mut snap_meta = String::new();

    for (_, snapshot) in snapshots {
        match newest_timestamp {
            Some(current_newest) if snapshot.timestamp <= current_newest => continue,
            _ => {
                newest_timestamp = Some(snapshot.timestamp);
                snap_meta = snapshot.hash.clone();
            }
        }
    }

    if snap_meta.is_empty() {
        return Err(Errors::NoMatches);
    }

    Ok(snap_meta)
}
