use std::{
    collections::{HashMap, HashSet},
    env, fs,
    io::Read,
    path::Path,
};

use zstd::Decoder;

use crate::utils::{
    CellRef, Errors, MainManifest, ProjectManifest, ProjectRef, Snapshot, context::AppContext,
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
        copy_all(&manifest, ctx, dest, &mut copied)?;
        return Ok(());
    } else if cell == None && project_name != "all" {
        copy_project(ctx, &mut manifest, project_name, dest, &mut copied)?;
        return Ok(());
    }

    let cell_name = cell.ok_or(Errors::InternalError)?;
    copy_cell(ctx, &mut manifest, project_name, cell_name, dest)?;
    Ok(())
}

fn copy_cell(
    ctx: &AppContext,
    manifest: &mut MainManifest,
    project_name: String,
    cell_name: String,
    root: &Path,
) -> Result<(), Errors> {
    let proj_ref = manifest
        .projects
        .remove(&project_name)
        .ok_or(Errors::ProjectNotFound(project_name.clone()))?;
    let uuid = proj_ref.manifest.clone();
    let mut project_manifest: ProjectManifest = ctx.load_project_manifest(uuid.clone())?;

    let new_proj_ref: ProjectRef = ProjectRef {
        path: proj_ref.path,
        manifest: proj_ref.manifest.clone(),
        latest: String::new(),
        cells: vec![cell_name.clone()],
    };
    let mut new_proj = HashMap::new();
    new_proj.insert(project_name, new_proj_ref);
    let new_manifest: MainManifest = MainManifest {
        projects: new_proj,
        templates: HashMap::new(),
    };

    let cell_ref = project_manifest
        .cells
        .remove(&cell_name)
        .ok_or(Errors::InternalError)?;

    let mut new_cells: HashMap<String, CellRef> = HashMap::new();
    new_cells.insert(cell_name.clone(), cell_ref);

    let new_proj_manifest: ProjectManifest = ProjectManifest {
        source: project_manifest.source,
        description: project_manifest.description,
        timestamp: project_manifest.timestamp,
        snapshots: HashMap::new(),
        cells: new_cells,
    };

    let mut copied = HashSet::new();

    for (_, snapshot) in &new_proj_manifest
        .cells
        .get(&cell_name)
        .ok_or(Errors::InternalError)?
        .snapshots
    {
        copy_tree(
            ctx,
            copy_snapshot(ctx, snapshot.hash.clone(), root)?,
            root,
            &mut copied,
        )?;
    }

    write_project_manifest(uuid, &new_proj_manifest, root)?;
    write_main_manifest(&new_manifest, root)?;
    Ok(())
}

fn copy_project(
    ctx: &AppContext,
    manifest: &mut MainManifest,
    project_name: String,
    root: &Path,
    copied: &mut HashSet<String>,
) -> Result<(), Errors> {
    let proj_in_main = manifest
        .projects
        .remove(&project_name)
        .ok_or(Errors::ProjectNotFound(project_name.clone()))?;
    let uuid = proj_in_main.manifest.clone();
    let project_manifest: ProjectManifest = ctx.load_project_manifest(uuid.clone())?;
    let mut manifest_obj: MainManifest = MainManifest {
        projects: HashMap::new(),
        templates: HashMap::new(),
    };

    for (_, snapshot) in &project_manifest.snapshots {
        copy_tree(
            ctx,
            copy_snapshot(ctx, snapshot.hash.clone(), root)?,
            root,
            copied,
        )?;
    }

    for cell in &proj_in_main.cells {
        for (_, snapshot) in &project_manifest
            .cells
            .get(cell)
            .ok_or(Errors::InternalError)?
            .snapshots
        {
            copy_tree(
                ctx,
                copy_snapshot(ctx, snapshot.hash.clone(), root)?,
                root,
                copied,
            )?;
        }
    }
    manifest_obj.projects.insert(project_name, proj_in_main);
    copy_project_manifest(&ctx.project_manifest_path(uuid.clone()), uuid, root)?;
    write_main_manifest(&manifest_obj, root)?;
    Ok(())
}

fn copy_all(
    manifest: &MainManifest,
    ctx: &AppContext,
    root: &Path,
    copied: &mut HashSet<String>,
) -> Result<(), Errors> {
    for (_, project_ref) in &manifest.projects {
        let uuid = project_ref.manifest.clone();
        let project_manifest: ProjectManifest = ctx.load_project_manifest(uuid.clone())?;

        for (_, snapshot) in &project_manifest.snapshots {
            copy_tree(
                ctx,
                copy_snapshot(ctx, snapshot.hash.clone(), root)?,
                root,
                copied,
            )?;
        }

        for cell in &project_ref.cells {
            for (_, snapshot) in &project_manifest
                .cells
                .get(cell)
                .ok_or(Errors::InternalError)?
                .snapshots
            {
                copy_tree(
                    ctx,
                    copy_snapshot(ctx, snapshot.hash.clone(), root)?,
                    root,
                    copied,
                )?;
            }
        }
        copy_project_manifest(&ctx.project_manifest_path(uuid.clone()), uuid, root)?;
    }
    write_main_manifest(&manifest, root)?;
    Ok(())
}

fn copy_tree(
    ctx: &AppContext,
    hash: String,
    path: &Path,
    copied: &mut HashSet<String>,
) -> Result<(), Errors> {
    let dir = &hash[..3];
    let filename = &hash[3..];

    let file_path = ctx.objects_path().join(dir).join(filename);
    let mut file = fs::File::open(file_path)?;
    let mut blob_comp = Vec::new();
    file.read_to_end(&mut blob_comp)?;

    let mut content = Vec::new();
    {
        let mut decoder = Decoder::new(&blob_comp[..])?;
        decoder.read_to_end(&mut content)?;
    }

    let entries = parse_tree(&content)?;

    for entry in entries {
        if copied.contains(&hex::encode(entry.hash))
            || FileType::from_mode(u32::from_be_bytes(entry.mode)) == FileType::Cell
        {
            continue;
        }
        copied.insert(hex::encode(entry.hash));
        if FileType::from_mode(u32::from_be_bytes(entry.mode)) == FileType::Directory {
            copy_tree(ctx, hex::encode(entry.hash), path, copied)?;
        } else {
            copy_object(ctx, hex::encode(entry.hash), path)?;
        }
    }

    copy_object(ctx, hash, path)?;

    Ok(())
}

fn copy_object(ctx: &AppContext, hash: String, path: &Path) -> Result<(), Errors> {
    let dir = &hash[..3];
    let filename = &hash[3..];

    let root = ctx.objects_path().join(dir).join(filename);
    let mut file = fs::File::open(root)?;
    let mut blob_comp = Vec::new();
    file.read_to_end(&mut blob_comp)?;

    let directory = path.join("objects").join(dir);
    if !directory.exists() {
        fs::create_dir(&directory)?;
    }
    let filepath = directory.join(filename);

    fs::write(filepath, blob_comp)?;

    Ok(())
}

fn copy_snapshot(ctx: &AppContext, hash: String, path: &Path) -> Result<String, Errors> {
    let dir = &hash[..3];
    let filename = &hash[3..];

    let root = ctx.snapshots_path().join(dir).join(filename);

    let mut file = fs::File::open(root)?;
    let mut blob_comp = Vec::new();
    file.read_to_end(&mut blob_comp)?;

    let mut content = Vec::new();
    {
        let mut decoder = Decoder::new(&blob_comp[..])?;
        decoder.read_to_end(&mut content)?;
    }

    let snapshot: Snapshot = serde_json::from_slice(&content)?;
    let destination = path.join("snapshots").join(dir);
    fs::create_dir(&destination)?;
    let file = destination.join(filename);
    fs::write(file, blob_comp)?;
    Ok(snapshot.root)
}

fn copy_project_manifest(manifest: &Path, uuid: String, path: &Path) -> Result<(), Errors> {
    let project_manifest_data = fs::read(manifest)?;

    let file = path.join("projects").join(format!("{}.json", uuid));
    fs::write(file, project_manifest_data)?;

    Ok(())
}

fn write_project_manifest(
    uuid: String,
    manifest: &ProjectManifest,
    path: &Path,
) -> Result<(), Errors> {
    let project_manifest_path = path.join("projects").join(format!("{}.json", uuid));
    let project_manifest_data = serde_json::to_vec_pretty(manifest)?;
    fs::write(project_manifest_path, project_manifest_data)?;
    Ok(())
}

fn write_main_manifest(manifest: &MainManifest, path: &Path) -> Result<(), Errors> {
    let manifest_vec = serde_json::to_vec_pretty(manifest)?;
    fs::remove_file(path.join("manifest.json"))?;
    fs::write(path.join("manifest.json"), manifest_vec)?;
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
