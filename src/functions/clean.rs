use std::{collections::HashSet, fs, io::Read};

use zstd::Decoder;

use crate::utils::{Errors, MainManifest, ProjectManifest, Snapshot, denali_root};

pub fn clean(is_dry: bool) -> Result<(), Errors> {
    let mut objects = HashSet::new();
    let mut snapshots: HashSet<String> = HashSet::new();
    mark_entries(&mut snapshots, &mut objects)?;
    if is_dry {
        println!("The next entries are going to be deleted");
        for entry in snapshots {
            println!("{}", entry);
        }
    } else {
        delete_snapshots(&snapshots)?;
        delete_objects(&objects)?;
    }
    Ok(())
}

fn delete_objects(objects: &HashSet<String>) -> Result<(), Errors> {
    let path = denali_root().join("objects");

    for dir_entry in fs::read_dir(&path)? {
        let dir_entry = dir_entry?;
        if !dir_entry.file_type()?.is_dir() {
            continue;
        }

        for file_entry in fs::read_dir(dir_entry.path())? {
            let file_entry = file_entry?;
            let dir_name = dir_entry.file_name().to_string_lossy().to_string();
            let file_name = file_entry.file_name().to_string_lossy().to_string();

            let full_hash = format!("{}{}", dir_name, file_name);

            if !objects.contains(&full_hash) {
                fs::remove_file(file_entry.path())?;
            }
        }

        if fs::read_dir(dir_entry.path())?.next().is_none() {
            fs::remove_dir(dir_entry.path())?;
        }
    }
    Ok(())
}

fn delete_snapshots(snapshots: &HashSet<String>) -> Result<(), Errors> {
    let path = denali_root().join("snapshots").join("meta");

    for dir_entry in fs::read_dir(&path)? {
        let dir_entry = dir_entry?;
        if !dir_entry.file_type()?.is_dir() {
            continue;
        }

        for file_entry in fs::read_dir(dir_entry.path())? {
            let file_entry = file_entry?;
            let dir_name = dir_entry.file_name().to_string_lossy().to_string();
            let file_name = file_entry.file_name().to_string_lossy().to_string();

            let full_hash = format!("{}{}", dir_name, file_name);

            if snapshots.contains(&full_hash) {
                fs::remove_file(file_entry.path())?;
            }
        }

        if fs::read_dir(dir_entry.path())?.next().is_none() {
            fs::remove_dir(dir_entry.path())?;
        }
    }
    Ok(())
}

fn mark_entries(
    snapshots: &mut HashSet<String>,
    objects: &mut HashSet<String>,
) -> Result<(), Errors> {
    let mut good_entries: HashSet<String> = HashSet::new();
    let manifest = load_manifest()?;
    for (_, project_ref) in &manifest.projects {
        let project_manifest = load_project_manifest(project_ref.manifest.clone())?;
        for (_, snapshot) in &project_manifest.snapshots {
            good_entries.insert(snapshot.hash.clone());
        }
        for (_, cell_ref) in &project_manifest.cells {
            for (_, snapshot) in &cell_ref.snapshots {
                good_entries.insert(snapshot.hash.clone());
            }
        }
    }

    mark_orphans(snapshots, objects, &good_entries)?;

    Ok(())
}

fn mark_orphans(
    snapshots: &mut HashSet<String>,
    objects: &mut HashSet<String>,
    good_entries: &HashSet<String>,
) -> Result<(), Errors> {
    let path = denali_root().join("snapshots").join("meta");
    for entry in fs::read_dir(path)? {
        let dir = entry?;
        for file in fs::read_dir(dir.path())? {
            let dir_name = dir.file_name().to_string_lossy().to_string();
            let filename = file?.file_name().to_string_lossy().to_string();
            let full_hash = format!("{}{}", dir_name, filename);
            if !good_entries.contains(&full_hash) {
                snapshots.insert(full_hash);
            }
        }
    }
    for snapshot in good_entries.iter() {
        let snap = read_snapshot(snapshot.to_string())?;
        mark_objects(&snap.root, objects, &snapshots)?;
    }
    Ok(())
}

fn mark_objects(
    hash: &str,
    good_entries: &mut HashSet<String>,
    bad_snapshots: &HashSet<String>,
) -> Result<(), Errors> {
    let dir = &hash[..3];
    let file = &hash[3..];
    let path = denali_root().join("objects").join(dir).join(file);
    if !path.exists() {
        return Ok(());
    }
    let mut file = fs::File::open(path)?;
    let mut tree_cmp = Vec::new();
    file.read_to_end(&mut tree_cmp)?;

    let mut tree = Vec::new();
    {
        let mut decoder = Decoder::new(&tree_cmp[..])?;
        decoder.read_to_end(&mut tree)?;
    }

    let entries = parse_tree(&tree)?;

    good_entries.insert(hash.to_string());

    for entry in entries {
        if entry.mode == "20" {
            good_entries.insert(hex::encode(entry.hash));
        } else if entry.mode == "30" {
            let snap = read_snapshot(hex::encode(entry.hash))?;
            if !bad_snapshots.contains(&snap.root) {
                mark_objects(&snap.root, good_entries, bad_snapshots)?;
            }
        } else {
            mark_objects(&hex::encode(entry.hash), good_entries, bad_snapshots)?;
        }
    }
    Ok(())
}

struct TreeStruct {
    mode: String,
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

fn read_snapshot(hash: String) -> Result<Snapshot, Errors> {
    let dir = &hash[..3];
    let filename = &hash[3..];

    let meta_path = denali_root()
        .join("snapshots")
        .join("meta")
        .join(dir)
        .join(filename);
    let meta_data_cmp = fs::read(meta_path)?;
    let mut meta_data = Vec::new();
    {
        let mut decoder = Decoder::new(&meta_data_cmp[..])?;
        decoder.read_to_end(&mut meta_data)?;
    }

    let meta: Snapshot = serde_json::from_slice(&meta_data)?;

    Ok(meta)
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
