use std::{
    collections::{HashMap, HashSet},
    env, fs,
    io::Read,
    ops::Deref,
    path::Path,
};

use zstd::Decoder;

use crate::utils::{
    CellRef, Errors, MainManifest, ProjectManifest, ProjectRef, Snapshot, denali_root,
    root_dir::make_root_dir,
};

pub fn copy(project: String, path: Option<&Path>) -> Result<(), Errors> {
    let mut copied: HashSet<String> = HashSet::new();

    let mut parts = project.split('@');
    let cell = parts.next().map(|s| s.to_string());
    let proj_name = parts.next().map(|s| s.to_string());

    let (cell, project_name) = match (cell, proj_name) {
        (Some(cell), Some(proj)) => (Some(cell), proj),
        (Some(proj), None) => (None, proj),
        _ => return Err(Errors::InvalidNameFormat(project)),
    };

    let dir = match path {
        Some(p) => env::current_dir()?.join(p),
        None => env::current_dir()?,
    };

    if !dir.exists() {
        return Err(Errors::DoesntExist(dir));
    } else if !dir.is_dir() {
        return Err(Errors::NotADir(dir));
    }

    let root = denali_root();
    let manifest_path = root.join("manifest.json");
    let manifest_data = fs::read(&manifest_path)?;
    let mut manifest: MainManifest = serde_json::from_slice(&manifest_data)?;

    make_root_dir(dir.join(".denali"))?;
    let root = &dir.join(".denali");

    if cell == None && project_name == "all" {
        for (_, project_ref) in &manifest.projects {
            let uuid = project_ref.manifest.clone();
            let project_manifest_path = denali_root()
                .join("snapshots")
                .join("projects")
                .join(format!("{}.json", uuid));
            let project_manifest_data = fs::read(project_manifest_path)?;
            let project_manifest: ProjectManifest = serde_json::from_slice(&project_manifest_data)?;

            for (_, snapshot) in &project_manifest.snapshots {
                copy_tree(
                    copy_snapshot(snapshot.hash.clone(), root)?,
                    root,
                    &mut copied,
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
                        copy_snapshot(snapshot.hash.clone(), root)?,
                        root,
                        &mut copied,
                    )?;
                }
            }
            copy_project_manifest(uuid, root)?;
        }
        write_main_manifest(&manifest, root)?;
        return Ok(());
    } else if cell == None && project_name != "all" {
        let proj_in_main = manifest
            .projects
            .remove(&project_name)
            .ok_or(Errors::ProjectNotFound(project_name.clone()))?;
        let uuid = proj_in_main.manifest.clone();
        let project_manifest_path = denali_root()
            .join("snapshots")
            .join("projects")
            .join(format!("{}.json", uuid));
        let project_manifest_data = fs::read(project_manifest_path)?;
        let project_manifest: ProjectManifest = serde_json::from_slice(&project_manifest_data)?;
        let mut manifest_obj: MainManifest = MainManifest {
            projects: HashMap::new(),
            templates: HashMap::new(),
        };

        for (_, snapshot) in &project_manifest.snapshots {
            copy_tree(
                copy_snapshot(snapshot.hash.clone(), root)?,
                root,
                &mut copied,
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
                    copy_snapshot(snapshot.hash.clone(), root)?,
                    root,
                    &mut copied,
                )?;
            }
        }
        manifest_obj.projects.insert(project_name, proj_in_main);
        copy_project_manifest(uuid, root)?;
        write_main_manifest(&manifest_obj, root)?;
        return Ok(());
    }

    let cell_name = cell.ok_or(Errors::InternalError)?;
    let proj_ref = manifest
        .projects
        .remove(&project_name)
        .ok_or(Errors::ProjectNotFound(project_name.clone()))?;
    let uuid = proj_ref.manifest.clone();
    let project_manifest_path = denali_root()
        .join("snapshots")
        .join("projects")
        .join(format!("{}.json", uuid));
    let project_manifest_data = fs::read(project_manifest_path)?;
    let mut project_manifest: ProjectManifest = serde_json::from_slice(&project_manifest_data)?;

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
            copy_snapshot(snapshot.hash.clone(), root)?,
            root,
            &mut copied,
        )?;
    }

    write_project_manifest(uuid, &new_proj_manifest, root)?;
    write_main_manifest(&new_manifest, root)?;

    Ok(())
}

fn copy_tree(hash: String, path: &Path, copied: &mut HashSet<String>) -> Result<(), Errors> {
    let dir = &hash[..3];
    let filename = &hash[3..];

    let file_path = Path::new("objects").join(dir).join(filename);
    let root = denali_root().join(file_path.clone());
    let mut file = fs::File::open(root)?;
    let mut blob_comp = Vec::new();
    file.read_to_end(&mut blob_comp)?;

    let mut content = Vec::new();
    {
        let mut decoder = Decoder::new(&blob_comp[..])?;
        decoder.read_to_end(&mut content)?;
    }

    let entries = parse_tree(&content)?;

    for entry in entries {
        if copied.contains(&hex::encode(entry.hash)) || entry.mode == "30" {
            continue;
        }
        if entry.mode == "10" {
            copy_tree(hex::encode(entry.hash), path, copied)?;
        } else {
            copy_object(hex::encode(entry.hash), path)?;
            copied.insert(hex::encode(entry.hash));
        }
    }

    copy_object(hash, path)?;

    Ok(())
}
fn copy_object(hash: String, path: &Path) -> Result<(), Errors> {
    let dir = &hash[..3];
    let filename = &hash[3..];

    let root = denali_root().join("objects").join(dir).join(filename);
    let mut file = fs::File::open(root)?;
    let mut blob_comp = Vec::new();
    file.read_to_end(&mut blob_comp)?;

    let directory = path.join("objects").join(dir);
    fs::create_dir(&directory)?;
    let filepath = directory.join(filename);

    fs::write(filepath, blob_comp)?;

    Ok(())
}

fn copy_snapshot(hash: String, path: &Path) -> Result<String, Errors> {
    let dir = &hash[..3];
    let filename = &hash[3..];

    let root = denali_root()
        .join("snapshots")
        .join("meta")
        .join(dir)
        .join(format!("{}.json.zstd", filename));

    let mut file = fs::File::open(root)?;
    let mut blob_comp = Vec::new();
    file.read_to_end(&mut blob_comp)?;

    let mut content = Vec::new();
    {
        let mut decoder = Decoder::new(&blob_comp[..])?;
        decoder.read_to_end(&mut content)?;
    }

    let snapshot: Snapshot = serde_json::from_slice(&content)?;
    let destination = path.join("snapshots").join("meta").join(dir);
    fs::create_dir(&destination)?;
    let file = destination.join(format!("{}.json.zstd", filename));
    fs::write(file, blob_comp)?;
    Ok(snapshot.root)
}

fn copy_project_manifest(uuid: String, path: &Path) -> Result<(), Errors> {
    let project_manifest_path = denali_root()
        .join("snapshots")
        .join("projects")
        .join(format!("{}.json", uuid));
    let project_manifest_data = fs::read(project_manifest_path)?;

    let file = path
        .join("snapshots")
        .join("projects")
        .join(format!("{}.json", uuid));
    fs::write(file, project_manifest_data)?;

    Ok(())
}

fn write_project_manifest(
    uuid: String,
    manifest: &ProjectManifest,
    path: &Path,
) -> Result<(), Errors> {
    let project_manifest_path = path
        .join("snapshots")
        .join("projects")
        .join(format!("{}.json", uuid));
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
    mode: String,
    name: String,
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
