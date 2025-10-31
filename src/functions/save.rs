use chrono::Utc;
use globset::{Glob, GlobSet, GlobSetBuilder};

use crate::{
    objects::{make_tree, save_object::save_snapshot},
    utils::*,
};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

pub fn save(project: String, name: String, description: Option<&str>) -> Result<(), Errors> {
    let desc = description.unwrap_or("");

    let root = denali_root();
    let manifest_path = root.join("manifest.json");
    let manifest_data = fs::read(&manifest_path)?;
    let mut manifest: MainManifest = serde_json::from_slice(&manifest_data)?;

    let mut parts = project.split('@');
    let cell = parts.next().map(|s| s.to_string());
    let project = parts.next().map(|s| s.to_string());

    let (cell, project) = match (cell, project) {
        (Some(cell), Some(project)) => (Some(cell), project),
        (Some(project), None) => (None, project),
        _ => return Err(Errors::InvalidNameFormat(name)),
    };

    let proj = manifest
        .projects
        .get(&project)
        .ok_or_else(|| Errors::NotInitialised(PathBuf::from(&project)))?;

    if let Some(cell_name) = &cell {
        if !proj.cells.contains(cell_name) {
            return Err(Errors::NotInitialised(PathBuf::from(cell_name)));
        }
    }

    let uuid = &manifest
        .projects
        .get(&project)
        .ok_or(Errors::InternalError)?
        .manifest;

    if cell == None {
        let hash_list = make_project_save(
            desc,
            uuid,
            &manifest
                .projects
                .get(&project)
                .ok_or(Errors::InternalError)?
                .cells,
        )?;
        update_all_manifests(&name, &project, &mut manifest, hash_list)?;
        return Ok(());
    }

    save_cell(
        &name,
        &cell.ok_or(Errors::InternalError)?,
        desc,
        manifest
            .projects
            .get(&project)
            .ok_or(Errors::InternalError)?
            .manifest
            .clone(),
    )?;
    Ok(())
}

fn save_cell(name: &str, cell: &str, description: &str, uuid: String) -> Result<(), Errors> {
    let file_path = denali_root()
        .join("snapshots")
        .join("projects")
        .join(format!("{}.json", uuid));
    let manifest_data = fs::read(&file_path)?;
    let mut project_manifest: ProjectManifest = serde_json::from_slice(&manifest_data)?;

    if project_manifest.snapshots.contains_key(name) {
        return Err(Errors::SnapshotExists(name.to_string()));
    }

    let toml_file = Path::new(&project_manifest.source).join(".denali.toml");
    let data = fs::read_to_string(&toml_file)?;
    let config: DenaliToml = toml::from_str(&data)?;
    let ignore = &config.cells.get(cell).ok_or(Errors::InternalError)?.ignore;
    let glob = build_globset(&ignore)?;

    let hash = hash_dir(
        Path::new(
            &project_manifest
                .cells
                .get(cell)
                .ok_or(Errors::InternalError)?
                .path,
        ),
        &glob,
        description,
        &HashMap::new(),
    )?;

    if let Some(cell_ref) = project_manifest.cells.get_mut(cell) {
        let hex = hex::encode(hash);
        let snapshot: Snapshots = Snapshots {
            hash: hex.clone(),
            timestamp: Utc::now(),
        };
        cell_ref.latest = hex;
        cell_ref.snapshots.insert(name.to_string(), snapshot);
    } else {
        return Err(Errors::InternalError);
    }
    let project_data = serde_json::to_vec_pretty(&project_manifest)?;

    fs::write(file_path, project_data)?;

    Ok(())
}

fn build_globset(patterns: &[String]) -> Result<GlobSet, Errors> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        builder.add(Glob::new(pattern)?);
    }
    Ok(builder.build()?)
}

fn make_project_save(
    description: &str,
    uuid: &String,
    cells: &Vec<String>,
) -> Result<HashMap<String, [u8; 32]>, Errors> {
    let data = fs::read(
        denali_root()
            .join("snapshots")
            .join("projects")
            .join(format!("{}.json", uuid)),
    )?;
    let proj_manifest: ProjectManifest = serde_json::from_slice(&data)?;
    let root_dir = &proj_manifest.source;
    let config_data = fs::read_to_string(Path::new(&root_dir).join(".denali.toml"))?;
    let config: DenaliToml = toml::from_str(&config_data)?;
    let mut cells_map: HashMap<String, PathBuf> = HashMap::new();
    let mut ignore_cells: HashMap<String, GlobSet> = HashMap::new();
    let mut root_ignore = config.root.ignore;
    for cell in cells {
        let path = proj_manifest
            .cells
            .get(cell)
            .ok_or(Errors::InternalError)?
            .path
            .clone();
        cells_map.insert(cell.clone(), Path::new(&path).to_path_buf());
        ignore_cells.insert(
            cell.clone(),
            build_globset(&config.cells.get(cell).ok_or(Errors::InternalError)?.ignore)?,
        );

        if Path::new(&path).starts_with(&root_dir) {
            if let Some(name) = Path::new(&path).file_name() {
                root_ignore.push(name.to_string_lossy().to_string());
            }
        }
    }

    Ok(save_project(
        description,
        &Path::new(&proj_manifest.source),
        &build_globset(&root_ignore)?,
        cells_map,
        ignore_cells,
    )?)
}

pub fn update_all_manifests(
    name: &str,
    project: &str,
    manifest: &mut MainManifest,
    hash_list: HashMap<String, [u8; 32]>,
) -> Result<(), Errors> {
    let uuid = manifest
        .projects
        .get(project)
        .ok_or(Errors::InternalError)?
        .manifest
        .clone();

    let file_path = denali_root()
        .join("snapshots")
        .join("projects")
        .join(format!("{}.json", uuid));
    let manifest_data = fs::read(&file_path)?;
    let mut project_manifest: ProjectManifest = serde_json::from_slice(&manifest_data)?;

    let mut root_hash: Option<String> = None;

    for (entry, hash) in &hash_list {
        let hash_hex = hex::encode(hash);

        if entry == "root" {
            root_hash = Some(hash_hex.clone());
            let snapshot: Snapshots = Snapshots {
                hash: hash_hex,
                timestamp: Utc::now(),
            };
            project_manifest
                .snapshots
                .insert(name.to_string(), snapshot);
        } else {
            let entry_man = project_manifest
                .cells
                .get_mut(entry)
                .ok_or(Errors::InternalError)?;

            let snapshot: Snapshots = Snapshots {
                hash: hash_hex.clone(),
                timestamp: Utc::now(),
            };
            entry_man.snapshots.insert(name.to_string(), snapshot);
            entry_man.latest = hash_hex;
        }
    }

    if let Some(root_hash) = root_hash {
        if let Some(proj_ref) = manifest.projects.get_mut(project) {
            proj_ref.latest = root_hash;
        } else {
            return Err(Errors::InternalError);
        }
    }

    let root = denali_root();
    let manifest_path = root.join("manifest.json");
    let file_path = root
        .join("snapshots")
        .join("projects")
        .join(format!("{}.json", uuid));

    let mani_json = serde_json::to_vec_pretty(manifest)?;
    let proj_json = serde_json::to_vec_pretty(&project_manifest)?;

    fs::write(&manifest_path, mani_json)?;
    fs::write(&file_path, proj_json)?;

    Ok(())
}

fn save_project(
    description: &str,
    path: &Path,
    ignore: &GlobSet,
    cells: HashMap<String, PathBuf>,
    ignore_cells: HashMap<String, GlobSet>,
) -> Result<HashMap<String, [u8; 32]>, Errors> {
    let mut cells_hash: HashMap<String, [u8; 32]> = HashMap::new();

    for (cell, cell_path) in &cells {
        let hash = hash_dir(
            &cell_path,
            ignore_cells.get(cell).ok_or(Errors::InternalError)?,
            description,
            &HashMap::new(),
        )?;
        cells_hash.insert(cell.to_string(), hash);
    }

    let root_hash = hash_dir(path, ignore, description, &cells_hash)?;
    cells_hash.insert("root".to_string(), root_hash);
    Ok(cells_hash)
}

fn hash_dir(
    path: &Path,
    ignore: &GlobSet,
    description: &str,
    cells: &HashMap<String, [u8; 32]>,
) -> Result<[u8; 32], Errors> {
    let hash = make_tree(path, ignore, cells, path)?;

    let snapshot: Snapshot = Snapshot {
        description: description.to_string(),
        timestamp: Utc::now(),
        root: hex::encode(hash),
    };

    let content = serde_json::to_vec(&snapshot)?;

    Ok(save_snapshot(content)?)
}
