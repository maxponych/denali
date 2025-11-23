use chrono::Utc;
use globset::{Glob, GlobSet, GlobSetBuilder};

use crate::utils::{
    DenaliToml, Errors, MainManifest, ProjectManifest, Snapshot, Snapshots, TreeStruct,
    context::AppContext, parse_name,
};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::Read,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

pub fn save(
    ctx: &AppContext,
    project: String,
    name: String,
    description: Option<&str>,
) -> Result<(), Errors> {
    let desc = description.unwrap_or("");

    let mut manifest: MainManifest = ctx.load_main_manifest()?;

    let (project, cell) = parse_name(project)?;

    let proj = manifest
        .projects
        .get(&project)
        .ok_or_else(|| Errors::NotInitialised(PathBuf::from(&project)))?;

    if let Some(cell_name) = &cell {
        if !proj.cells.contains(cell_name) {
            return Err(Errors::NotInitialised(PathBuf::from(cell_name)));
        }
    }

    let uuid = manifest
        .projects
        .get(&project)
        .ok_or(Errors::InternalError)?
        .manifest
        .clone();

    if cell == None {
        let hash_list = make_project_save(
            ctx,
            uuid,
            desc,
            &manifest
                .projects
                .get(&project)
                .ok_or(Errors::InternalError)?
                .cells,
        )?;
        update_all_manifests(ctx, &name, &project, &mut manifest, hash_list)?;
        return Ok(());
    }

    save_cell(
        ctx,
        ctx.project_manifest_path(
            manifest
                .projects
                .get(&project)
                .ok_or(Errors::InternalError)?
                .manifest
                .clone(),
        ),
        &name,
        &cell.ok_or(Errors::InternalError)?,
        desc,
    )?;
    Ok(())
}

fn save_cell(
    ctx: &AppContext,
    manifest_path: PathBuf,
    name: &str,
    cell: &str,
    description: &str,
) -> Result<(), Errors> {
    let manifest_data = fs::read(&manifest_path)?;
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
        ctx,
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

    fs::write(manifest_path, project_data)?;

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
    ctx: &AppContext,
    uuid: String,
    description: &str,
    cells: &Vec<String>,
) -> Result<HashMap<String, ([u8; 32], [u8; 4])>, Errors> {
    let proj_manifest: ProjectManifest = ctx.load_project_manifest(uuid)?;
    let source_dir = &proj_manifest.source;
    let config_data = fs::read_to_string(Path::new(&source_dir).join(".denali.toml"))?;
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

        if Path::new(&path).starts_with(&source_dir) {
            if let Some(name) = Path::new(&path).file_name() {
                root_ignore.push(name.to_string_lossy().to_string());
            }
        }
    }

    Ok(save_project(
        ctx,
        description,
        &Path::new(&proj_manifest.source),
        &build_globset(&root_ignore)?,
        cells_map,
        ignore_cells,
    )?)
}

pub fn update_all_manifests(
    ctx: &AppContext,
    name: &str,
    project: &str,
    manifest: &mut MainManifest,
    hash_list: HashMap<String, ([u8; 32], [u8; 4])>,
) -> Result<(), Errors> {
    let uuid = manifest
        .projects
        .get(project)
        .ok_or(Errors::InternalError)?
        .manifest
        .clone();

    let manifest_data = fs::read(ctx.project_manifest_path(uuid.clone()))?;
    let mut project_manifest: ProjectManifest = serde_json::from_slice(&manifest_data)?;

    let mut root_hash: Option<String> = None;

    for (entry, hash) in &hash_list {
        let hash_hex = hex::encode(hash.0);

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

    let manifest_path = ctx.main_manifest_path();

    let mani_json = serde_json::to_vec_pretty(manifest)?;
    let proj_json = serde_json::to_vec_pretty(&project_manifest)?;

    fs::write(&manifest_path, mani_json)?;
    fs::write(ctx.project_manifest_path(uuid), proj_json)?;

    Ok(())
}

fn save_project(
    ctx: &AppContext,
    description: &str,
    path: &Path,
    ignore: &GlobSet,
    cells: HashMap<String, PathBuf>,
    ignore_cells: HashMap<String, GlobSet>,
) -> Result<HashMap<String, ([u8; 32], [u8; 4])>, Errors> {
    let mut cells_hash: HashMap<String, ([u8; 32], [u8; 4])> = HashMap::new();

    for (cell, cell_path) in &cells {
        let hash = hash_dir(
            ctx,
            &cell_path,
            ignore_cells.get(cell).ok_or(Errors::InternalError)?,
            description,
            &HashMap::new(),
        )?;
        let meta = fs::symlink_metadata(cell_path)?;
        let perms = meta.mode() & 0x0FFF;
        let custom_mode = 0xB000 | perms;
        let mode = custom_mode.to_be_bytes();

        cells_hash.insert(cell.to_string(), (hash, mode));
    }

    let root_hash = hash_dir(ctx, path, ignore, description, &cells_hash)?;
    cells_hash.insert("root".to_string(), (root_hash, [0, 0, 0, 0]));
    Ok(cells_hash)
}

fn hash_dir(
    ctx: &AppContext,
    path: &Path,
    ignore: &GlobSet,
    description: &str,
    cells: &HashMap<String, ([u8; 32], [u8; 4])>,
) -> Result<[u8; 32], Errors> {
    let hash = make_tree(ctx, path, ignore, cells, path)?;

    let meta = fs::symlink_metadata(path)?;
    let mode = meta.mode().to_be_bytes();

    let snapshot: Snapshot = Snapshot {
        description: description.to_string(),
        timestamp: Utc::now(),
        root: hex::encode(hash),
        permissions: mode,
    };

    let content = serde_json::to_vec(&snapshot)?;

    Ok(ctx.save_snapshot(content)?)
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

pub fn make_tree(
    ctx: &AppContext,
    path: &Path,
    ignore: &GlobSet,
    cells: &HashMap<String, ([u8; 32], [u8; 4])>,
    root_path: &Path,
) -> Result<[u8; 32], Errors> {
    let mut entries: Vec<TreeStruct> = Vec::new();

    for (name, hash) in cells {
        entries.push(TreeStruct {
            mode: hash.1.clone(),
            name: name.clone(),
            hash: hash.0.clone(),
        });
    }

    if path.is_dir() {
        if !fs::read_dir(path)?.next().is_none() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let path = entry.path();
                let name_os = path
                    .file_name()
                    .ok_or(Errors::DoesntExist(path.to_path_buf()))?;

                if ignore.is_match(path.strip_prefix(&root_path).unwrap_or(&path)) {
                    continue;
                }

                let meta = fs::symlink_metadata(path.clone()).unwrap();
                let mode = meta.mode().to_be_bytes();

                let hash = if meta.file_type().is_symlink() {
                    let target = fs::read_link(&path)?;
                    ctx.save_object(target.to_string_lossy().as_bytes().to_vec())?
                } else if meta.is_dir() {
                    make_tree(ctx, &path, &ignore, &HashMap::new(), root_path)?
                } else {
                    hash_file(ctx, &path)?
                };

                entries.push(TreeStruct {
                    mode,
                    name: name_os.to_string_lossy().to_string(),
                    hash,
                });
            }
        }
    } else {
        if !ignore.is_match(path.strip_prefix(&root_path).unwrap_or(&path)) {
            let name_os = path
                .file_name()
                .ok_or(Errors::DoesntExist(path.to_path_buf()))?;
            let hash = hash_file(ctx, path)?;
            let meta = fs::symlink_metadata(path).unwrap();
            let mode = meta.mode().to_be_bytes();
            entries.push(TreeStruct {
                mode,
                name: name_os.to_string_lossy().to_string(),
                hash,
            });
        }
    };

    let hash = build_tree(ctx, entries)?;
    Ok(hash)
}
