use globset::GlobSet;

use crate::objects::save_object;
use std::{
    collections::HashMap,
    ffi::OsString,
    fs::{self, File},
    io::Read,
    os::unix::ffi::OsStrExt,
    path::Path,
};

use crate::utils::*;

struct TreeStruct {
    mode: String,
    name: OsString,
    hash: [u8; 32],
}

fn build_tree(entries: Vec<TreeStruct>) -> Result<[u8; 32], Errors> {
    let mut content = Vec::new();

    for entry in entries {
        content.extend_from_slice(entry.mode.as_bytes());
        content.push(b' ');
        content.extend_from_slice(entry.name.as_bytes());
        content.push(0);
        content.extend_from_slice(&entry.hash);
    }

    let hash = save_object(content)?;
    Ok(hash)
}

fn hash_file(path: &Path) -> Result<[u8; 32], Errors> {
    let mut file = File::open(path)?;
    let mut content = Vec::new();
    file.read_to_end(&mut content)?;

    let hash = save_object(content)?;
    Ok(hash)
}

pub fn make_tree(
    path: &Path,
    ignore: &GlobSet,
    cells: &HashMap<String, [u8; 32]>,
    root: &Path,
) -> Result<[u8; 32], Errors> {
    let mut entries: Vec<TreeStruct> = Vec::new();

    let mode_dir = "10";
    let mode_file = "20";
    let mode_cell = "30";

    for (name, hash) in cells {
        entries.push(TreeStruct {
            mode: mode_cell.to_string(),
            name: OsString::from(name.clone()),
            hash: hash.clone(),
        });
    }

    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            let name_os = path
                .file_name()
                .ok_or(Errors::DoesntExist(path.to_path_buf()))?;

            if ignore.is_match(path.strip_prefix(&root).unwrap_or(&path)) {
                continue;
            }

            let hash = if path.is_dir() {
                make_tree(&path, &ignore, &HashMap::new(), root)?
            } else {
                hash_file(&path)?
            };

            entries.push(TreeStruct {
                mode: if path.is_dir() { mode_dir } else { mode_file }.to_string(),
                name: name_os.to_os_string(),
                hash,
            });
        }
    } else {
        if !ignore.is_match(path.strip_prefix(&root).unwrap_or(&path)) {
            let name_os = path
                .file_name()
                .ok_or(Errors::DoesntExist(path.to_path_buf()))?;
            let hash = hash_file(path)?;
            entries.push(TreeStruct {
                mode: mode_file.to_string(),
                name: name_os.to_os_string(),
                hash,
            });
        }
    };

    let hash = build_tree(entries)?;
    Ok(hash)
}
