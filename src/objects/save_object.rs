use crate::utils::*;
use blake3;
use std::fs;
use std::io::Write;
use zstd::stream::Encoder;

pub fn save_object(content: Vec<u8>) -> Result<[u8; 32], Errors> {
    let mut compressed = Vec::new();
    {
        let mut encoder = Encoder::new(&mut compressed, 3)?;
        encoder.write_all(&content)?;
        encoder.finish()?;
    }

    let hash = blake3::hash(&compressed);
    let name = hash.to_hex().to_string();
    let dir = &name[..3];
    let filename = &name[3..];

    let obj_path = denali_root().join("objects").join(dir);
    fs::create_dir_all(&obj_path)?;
    let file_path = obj_path.join(filename);

    fs::write(&file_path, &compressed)?;

    Ok(*hash.as_bytes())
}

pub fn save_snapshot(content: Vec<u8>) -> Result<[u8; 32], Errors> {
    let mut compressed = Vec::new();
    {
        let mut encoder = Encoder::new(&mut compressed, 3)?;
        encoder.write_all(&content)?;
        encoder.finish()?;
    }

    let hash = blake3::hash(&compressed);
    let name = hash.to_hex().to_string();
    let dir = &name[..2];
    let filename = &name[2..];

    let obj_path = denali_root().join("snapshots").join("meta").join(dir);
    fs::create_dir_all(&obj_path)?;
    let file_path = obj_path.join(format!("{}.json.zstd", filename));

    fs::write(&file_path, &compressed)?;

    Ok(*hash.as_bytes())
}
