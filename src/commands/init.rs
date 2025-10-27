use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub fn init(path: Option<&Path>) -> io::Result<()> {
    let dir: PathBuf = match path {
        Some(p) => p.join(".denali"),
        None => PathBuf::from(".denali"),
    };

    if dir.exists() && dir.is_dir() {
        println!("Repository already initialized.");
        return Ok(());
    }

    fs::create_dir(&dir)?;
    fs::create_dir(dir.join("objects"))?;
    fs::create_dir(dir.join("refs"))?;

    println!("Initialized empty Denali repository in {:?}", dir);
    Ok(())
}
