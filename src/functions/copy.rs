use std::{
    collections::{HashMap, HashSet},
    env,
    path::Path,
};

use crate::utils::{
    CellRef, Errors, MainManifest, ProjectManifest, ProjectRef, context::AppContext,
    file_type::FileType, parse_name,
};

pub fn copy(ctx: &AppContext, project: String, path: Option<&Path>) -> Result<(), Errors> {
    let mut copied: HashSet<String> = HashSet::new();

    let (project_name, cell) = parse_name(project)?;

    let dir = match path {
        Some(p) => env::current_dir()?.join(p),
        None => env::current_dir()?,
    };

    let dest = AppContext::new(Some(dir.clone()))?;

    if !dir.exists() {
        return Err(Errors::DoesntExist(dir));
    } else if !dir.is_dir() {
        return Err(Errors::NotADir(dir));
    }

    let mut manifest: MainManifest = ctx.load_main_manifest()?;

    dest.make_root_dir()?;

    if cell == None && project_name == "all" {
        copy_all(&manifest, ctx, &dest, &mut copied)?;
        return Ok(());
    } else if cell == None && project_name != "all" {
        copy_project(ctx, &mut manifest, project_name, &dest, &mut copied)?;
        return Ok(());
    }

    let cell_name = cell.ok_or(Errors::InternalError)?;
    copy_cell(ctx, &mut manifest, project_name, cell_name, &dest)?;
    Ok(())
}

fn copy_cell(
    ctx: &AppContext,
    manifest: &mut MainManifest,
    project_name: String,
    cell_name: String,
    dest: &AppContext,
) -> Result<(), Errors> {
    let proj_ref = manifest
        .projects
        .remove(&project_name)
        .ok_or(Errors::ProjectNotFound(project_name.clone()))?;
    if proj_ref.is_deleted {
        return Err(Errors::ProjectNotFound(project_name.clone()));
    }

    let uuid = proj_ref.manifest.clone();
    let mut project_manifest: ProjectManifest = ctx.load_project_manifest(uuid.clone())?;

    let cell_ref = project_manifest
        .cells
        .remove(&cell_name)
        .ok_or(Errors::InternalError)?;
    if cell_ref.is_deleted {
        return Err(Errors::ProjectNotFound(cell_name.clone()));
    }

    let new_proj_ref: ProjectRef = ProjectRef {
        path: proj_ref.path,
        is_deleted: proj_ref.is_deleted,
        timestamp: proj_ref.timestamp,
        manifest: proj_ref.manifest.clone(),
        latest: String::new(),
        cells: vec![cell_name.clone()],
    };
    let mut new_proj = HashMap::new();
    new_proj.insert(project_name, new_proj_ref);
    let new_manifest: MainManifest = MainManifest {
        projects: new_proj,
        remotes: HashMap::new(),
        templates: HashMap::new(),
    };

    let mut new_cells: HashMap<String, CellRef> = HashMap::new();
    new_cells.insert(cell_name.clone(), cell_ref.clone());

    let new_proj_manifest: ProjectManifest = ProjectManifest {
        source: project_manifest.source,
        name: project_manifest.name,
        description: project_manifest.description,
        timestamp: project_manifest.timestamp,
        snapshots: HashMap::new(),
        cells: new_cells,
    };

    let mut copied = HashSet::new();

    for (_, snapshot) in &cell_ref.snapshots {
        if snapshot.is_deleted {
            continue;
        }
        let snapshot = ctx.load_snapshot(snapshot.hash.clone())?;
        let bytes = serde_json::to_vec(&snapshot)?;
        dest.save_snapshot(bytes)?;
        copy_tree(ctx, snapshot.root, dest, &mut copied)?;
    }

    dest.write_project_manifest(uuid, &new_proj_manifest)?;
    dest.write_main_manifest(&new_manifest)?;
    Ok(())
}

fn copy_project(
    ctx: &AppContext,
    manifest: &mut MainManifest,
    project_name: String,
    dest: &AppContext,
    copied: &mut HashSet<String>,
) -> Result<(), Errors> {
    let proj_in_main = manifest
        .projects
        .remove(&project_name)
        .ok_or(Errors::ProjectNotFound(project_name.clone()))?;
    if proj_in_main.is_deleted {
        return Err(Errors::ProjectNotFound(project_name.clone()));
    }
    let uuid = proj_in_main.manifest.clone();
    let project_manifest: ProjectManifest = ctx.load_project_manifest(uuid.clone())?;
    let mut manifest_obj: MainManifest = MainManifest {
        projects: HashMap::new(),
        remotes: HashMap::new(),
        templates: HashMap::new(),
    };

    for (_, snapshot) in &project_manifest.snapshots {
        if snapshot.is_deleted {
            continue;
        }
        let snapshot = ctx.load_snapshot(snapshot.hash.clone())?;
        let bytes = serde_json::to_vec(&snapshot)?;
        dest.save_snapshot(bytes)?;
        copy_tree(ctx, snapshot.root, dest, copied)?;
    }

    for cell in &proj_in_main.cells {
        let cell = project_manifest
            .cells
            .get(cell)
            .ok_or(Errors::InternalError)?;
        if cell.is_deleted {
            continue;
        }
        for (_, snapshot) in &cell.snapshots {
            if snapshot.is_deleted {
                continue;
            }
            let snapshot = ctx.load_snapshot(snapshot.hash.clone())?;
            let bytes = serde_json::to_vec(&snapshot)?;
            dest.save_snapshot(bytes)?;
            copy_tree(ctx, snapshot.root, dest, copied)?;
        }
    }
    manifest_obj.projects.insert(project_name, proj_in_main);
    dest.write_project_manifest(uuid, &project_manifest)?;
    dest.write_main_manifest(&manifest_obj)?;
    Ok(())
}

fn copy_all(
    manifest: &MainManifest,
    ctx: &AppContext,
    dest: &AppContext,
    copied: &mut HashSet<String>,
) -> Result<(), Errors> {
    for (_, project_ref) in &manifest.projects {
        if project_ref.is_deleted {
            continue;
        }
        let uuid = project_ref.manifest.clone();
        let project_manifest: ProjectManifest = ctx.load_project_manifest(uuid.clone())?;

        for (_, snapshot) in &project_manifest.snapshots {
            if snapshot.is_deleted {
                continue;
            }
            if copied.contains(&snapshot.hash) {
                continue;
            }
            let snapshot = ctx.load_snapshot(snapshot.hash.clone())?;
            let bytes = serde_json::to_vec(&snapshot)?;
            dest.save_snapshot(bytes)?;
            copy_tree(ctx, snapshot.root, dest, copied)?;
        }

        for cell in &project_ref.cells {
            let cell = &project_manifest
                .cells
                .get(cell)
                .ok_or(Errors::InternalError)?;
            if cell.is_deleted {
                continue;
            }
            for (_, snapshot) in &cell.snapshots {
                if snapshot.is_deleted {
                    continue;
                }
                if copied.contains(&snapshot.hash) {
                    continue;
                }
                let snapshot = ctx.load_snapshot(snapshot.hash.clone())?;
                let bytes = serde_json::to_vec(&snapshot)?;
                dest.save_snapshot(bytes)?;
                copy_tree(ctx, snapshot.root, dest, copied)?;
            }
        }
        let proj_manifest = ctx.load_project_manifest(uuid.clone())?;
        dest.write_project_manifest(uuid, &proj_manifest)?;
    }
    dest.write_main_manifest(manifest)?;
    Ok(())
}

fn copy_tree(
    ctx: &AppContext,
    hash: String,
    dest: &AppContext,
    copied: &mut HashSet<String>,
) -> Result<(), Errors> {
    let content = ctx.load_object(hash)?;

    let entries = parse_tree(&content)?;

    for entry in entries {
        if copied.contains(&hex::encode(entry.hash)) {
            continue;
        }
        copied.insert(hex::encode(entry.hash));
        if FileType::from_mode(u32::from_be_bytes(entry.mode)) == FileType::Cell {
            let snapshot = ctx.load_snapshot(hex::encode(entry.hash))?;
            let bytes = serde_json::to_vec(&snapshot)?;
            dest.save_snapshot(bytes)?;
            copy_tree(ctx, snapshot.root, dest, copied)?;
        }
        if FileType::from_mode(u32::from_be_bytes(entry.mode)) == FileType::Directory {
            copy_tree(ctx, hex::encode(entry.hash), dest, copied)?;
        } else {
            let data = ctx.load_object(hex::encode(entry.hash))?;
            dest.save_object(data)?;
        }
    }

    dest.save_object(content)?;

    Ok(())
}

struct TreeStruct {
    mode: [u8; 4],
    hash: [u8; 32],
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

        while tree[i] != 0 {
            i += 1;
        }
        i += 1;

        let hash: [u8; 32] = tree[i..i + 32].try_into()?;
        i += 32;

        entries.push(TreeStruct {
            mode: mode,
            hash: hash,
        });
    }

    Ok(entries)
}
