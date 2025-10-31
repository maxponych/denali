use flate2::Compression;
use flate2::write::ZlibEncoder;
use hex;
use sha1::{Digest, Sha1};
use std::ffi::OsString;
use std::fs;
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

struct TreeStruct {
    mode: String,
    name: OsString,
    hash: [u8; 20],
}

fn save_object(content: Vec<u8>) -> io::Result<[u8; 20]> {
    let mut sha1 = Sha1::new();
    sha1.update(&content);
    let hash = sha1.finalize();

    let name = hex::encode(hash);
    let dir = &name[..2];
    let filename = &name[2..];

    let obj_path = Path::new(".denali/objects").join(dir);
    fs::create_dir_all(&obj_path).unwrap();
    let file_path = obj_path.join(filename);

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&content)?;
    let compressed = encoder.finish()?;

    let mut out = fs::File::create(file_path).unwrap();
    out.write_all(&compressed).unwrap();

    let hash_byts: [u8; 20] = hash.into();
    Ok(hash_byts)
}

fn hash_file(path: &Path) -> io::Result<[u8; 20]> {
    let mut file = File::open(path)?;
    let mut content = Vec::new();
    file.read_to_end(&mut content)?;

    let hash = save_object(content).unwrap();
    Ok(hash)
}

fn build_tree(entries: Vec<TreeStruct>) -> io::Result<[u8; 20]> {
    let mut content = Vec::new();

    for entry in entries {
        content.extend_from_slice(entry.mode.as_bytes());
        content.push(b' ');
        content.extend_from_slice(entry.name.as_bytes());
        content.push(0);
        content.extend_from_slice(&entry.hash);
    }

    let hash = save_object(content).unwrap();
    Ok(hash)
}

fn create_objects(path: &Path, ignore: &[String]) -> io::Result<[u8; 20]> {
    let mut entries: Vec<TreeStruct> = Vec::new();

    let mode_dir = "10";
    let mode_file = "20";

    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let path = entry?.path();
            let name = path.file_name().unwrap().to_string_lossy();

            if ignore
                .iter()
                .any(|rule| name == *rule || path.ends_with(rule))
            {
                continue;
            }

            if path.is_dir() {
                let hash = create_objects(&path, ignore).unwrap();
                entries.push(TreeStruct {
                    mode: mode_dir.to_string(),
                    name: path.file_name().unwrap().to_os_string(),
                    hash: hash,
                });
            } else {
                let hash = hash_file(&path).unwrap();
                entries.push(TreeStruct {
                    mode: mode_file.to_string(),
                    name: path.file_name().unwrap().to_os_string(),
                    hash: hash,
                });
            }
        }
    } else {
        let name = path.file_name().unwrap().to_string_lossy();
        if !ignore
            .iter()
            .any(|rule| name == *rule || path.ends_with(rule))
        {
            let hash = hash_file(path).unwrap();
            entries.push(TreeStruct {
                mode: mode_file.to_string(),
                name: path.file_name().unwrap().to_os_string(),
                hash: hash,
            });
        }
    }

    let hash = build_tree(entries).unwrap();
    Ok(hash)
}

pub fn save(path: &Path) -> io::Result<()> {
    let store = Path::new(".denali");

    if !store.exists() || !store.is_dir() {
        println!("Repository isn't initialised");
        return Ok(());
    }

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

    let hash = create_objects(path, &ignore).unwrap();

    let head_dir = store.join("refs").join("heads");
    fs::create_dir_all(&head_dir).unwrap();
    let head = head_dir.join("main");
    let hex = hex::encode(hash);
    fs::write(&head, hex.as_bytes()).unwrap();

    let head_path = Path::new("refs/heads");
    let head_file = store.join("HEAD");
    let mut head_content: Vec<u8> = Vec::new();
    let head_text = format!("ref: {}", head_path.join("main").to_str().unwrap());
    head_content.extend_from_slice(head_text.as_bytes());
    fs::write(head_file, &head_content).unwrap();

    Ok(())
}
