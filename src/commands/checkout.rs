use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::{fs, io};

use flate2::read::ZlibDecoder;

struct TreeStruct {
    mode: String,
    name: String,
    hash: [u8; 20],
}

fn restore_file(hash: String, dest: &Path) -> io::Result<()> {
    let path = Path::new(".denali/objects")
        .join(&hash[..2])
        .join(&hash[2..]);
    let mut file = fs::File::open(path)?;
    let mut blob_comp = Vec::new();
    file.read_to_end(&mut blob_comp)?;

    let mut decoder = ZlibDecoder::new(&blob_comp[..]);
    let mut blob = Vec::new();
    decoder.read_to_end(&mut blob).unwrap();

    let mut i = 0;
    while blob[i] != 0 {
        i += 1;
    }
    i += 1;

    let content = &blob[i..];

    let mut out = fs::File::create(dest).unwrap();
    out.write_all(&content).unwrap();

    Ok(())
}

fn parse_tree(tree: &Vec<u8>) -> io::Result<Vec<TreeStruct>> {
    let mut entries = Vec::new();

    let mut i = 0;
    while tree[i] != 0 {
        i += 1
    }
    i += 1;
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

        let hash: [u8; 20] = tree[i..i + 20].try_into().unwrap();
        i += 20;

        entries.push(TreeStruct {
            mode: mode,
            name: name,
            hash: hash,
        });
    }

    Ok(entries)
}

fn restore(hash: String, dest: &Path) -> io::Result<()> {
    let path = Path::new(".denali/objects")
        .join(&hash[..2])
        .join(&hash[2..]);
    let mut file = fs::File::open(path)?;
    let mut tree_comp = Vec::new();
    file.read_to_end(&mut tree_comp)?;

    let mut decoder = ZlibDecoder::new(&tree_comp[..]);
    let mut tree = Vec::new();
    decoder.read_to_end(&mut tree).unwrap();

    let entries = parse_tree(&tree).unwrap();

    for entry in entries {
        let target = dest.join(entry.name);

        if entry.mode == "40000" {
            fs::create_dir(&target)?;
            restore(hex::encode(entry.hash), &target)?;
        } else {
            restore_file(hex::encode(entry.hash), &target)?;
        }
    }

    Ok(())
}

fn wipe_dir(dir: &Path, ignore: Vec<String>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        let name = path.file_name().unwrap().to_string_lossy();

        if ignore
            .iter()
            .any(|rule| name == *rule || path.ends_with(rule))
        {
            continue;
        }

        if path.is_dir() {
            fs::remove_dir_all(&path)?;
        } else {
            fs::remove_file(&path)?;
        }
    }

    Ok(())
}

pub fn checkout(branch: String, dest: Option<&Path>) -> io::Result<()> {
    let head = Path::new(".denali/refs/heads").join(branch);
    let dest_dir = match dest {
        Some(d) if d.exists() && d.is_dir() => d,
        Some(d) => {
            eprintln!("Destination directory doesn't exist: {}", d.display());
            return Ok(());
        }
        None => &std::env::current_dir()?,
    };

    if head.exists() && head.is_file() {
        let ignore_file = Path::new(".denaliignore");

        let mut ignore: Vec<String> = vec![".denali".into()];

        if ignore_file.exists() && ignore_file.is_file() {
            let data = fs::read_to_string(ignore_file)?;
            for line in data.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                ignore.push(trimmed.to_string());
            }
        }

        wipe_dir(dest_dir, ignore).unwrap();

        let mut file = fs::File::open(&head)?;
        let mut hash = Vec::new();
        file.read_to_end(&mut hash)?;
        let hex = String::from_utf8_lossy(&hash).to_string();
        restore(hex, dest_dir)
    } else {
        println!(
            "No head with the name: {:?}",
            head.file_name().unwrap().to_str()
        );
        Ok(())
    }
}
