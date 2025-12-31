use std::{collections::HashSet, io::Write};

use zstd::Encoder;

use crate::utils::{Errors, TreeStruct, context::AppContext, file_type::FileType};

use super::PackType;

pub fn pack_snapshot(ctx: &AppContext, hash: &[u8; 32], pack: &mut Vec<u8>) -> Result<(), Errors> {
    let snapshot = ctx.load_snapshot(hex::encode(hash))?;
    let bytes = serde_json::to_vec(&snapshot)?;
    let size = (bytes.len() as u64).to_be_bytes();
    pack.push(PackType::Snapshot.as_byte());
    pack.extend_from_slice(hash);
    pack.extend_from_slice(&size);
    pack.extend_from_slice(&bytes);
    Ok(())
}

pub fn pack_object(ctx: &AppContext, hash: &[u8; 32], pack: &mut Vec<u8>) -> Result<(), Errors> {
    let bytes = ctx.load_object(hex::encode(hash))?;
    let size = (bytes.len() as u64).to_be_bytes();
    pack.push(PackType::Object.as_byte());
    pack.extend_from_slice(hash);
    pack.extend_from_slice(&size);
    pack.extend_from_slice(&bytes);
    Ok(())
}

pub fn pack_tree(
    ctx: &AppContext,
    hash: String,
    pack: &mut Vec<u8>,
    copied: &mut HashSet<String>,
) -> Result<(), Errors> {
    let tree = ctx.load_object(hash.clone())?;

    let entries = parse_tree(&tree)?;

    for entry in entries {
        let mode = u32::from_be_bytes(entry.mode);
        let filetype = FileType::from_mode(mode);
        let hash_str = hex::encode(entry.hash);
        match filetype {
            FileType::Directory => {
                if !copied.contains(&hash_str) {
                    pack_tree(ctx, hash_str.clone(), pack, copied)?;
                    copied.insert(hash_str);
                }
            }
            FileType::Cell => {
                if !copied.contains(&hash_str) {
                    pack_snapshot(ctx, &entry.hash, pack)?;
                    copied.insert(hash_str.clone());
                    let snapshot = ctx.load_snapshot(hash_str)?;
                    if !copied.contains(&snapshot.root) {
                        pack_tree(ctx, snapshot.root.clone(), pack, copied)?;
                        copied.insert(snapshot.root);
                    }
                }
            }
            _ => {
                if !copied.contains(&hash_str) {
                    pack_object(ctx, &entry.hash, pack)?;
                    copied.insert(hash_str);
                }
            }
        }
    }

    let mut hash_bytes = [0u8; 32];
    hex::decode_to_slice(hash, &mut hash_bytes)?;

    pack_object(ctx, &hash_bytes, pack)?;

    Ok(())
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

        let name_start = i;
        while tree[i] != 0 {
            i += 1;
        }
        let name = String::from_utf8_lossy(&tree[name_start..i]).to_string();
        i += 1;

        let hash: [u8; 32] = tree[i..i + 32].try_into()?;
        i += 32;

        entries.push(TreeStruct {
            mode,
            name: name,
            hash,
        });
    }

    Ok(entries)
}

pub fn unpack_object(ctx: &AppContext, content: &[u8], pointer: &mut u64) -> Result<(), Errors> {
    let mut i = *pointer as usize;
    let hash: [u8; 32] = content[i..i + 32].try_into()?;
    i += 32;
    let size = u64::from_be_bytes(content[i..i + 8].try_into()?);
    i += 8;
    let bytes = &content[i..i + size as usize];
    i += size as usize;
    *pointer = i as u64;

    let mut compressed = Vec::new();
    {
        let mut encoder = Encoder::new(&mut compressed, 3)?;
        encoder.write_all(&bytes)?;
        encoder.finish()?;
    }

    let check = *blake3::hash(&compressed).as_bytes();

    if hash != check {
        return Err(Errors::HashMismatch);
    }

    ctx.save_object(bytes.to_vec())?;
    Ok(())
}

pub fn unpack_snapshot(ctx: &AppContext, content: &[u8], pointer: &mut u64) -> Result<(), Errors> {
    let mut i = *pointer as usize;
    let hash: [u8; 32] = content[i..i + 32].try_into()?;
    i += 32;
    let size = u64::from_be_bytes(content[i..i + 8].try_into()?);
    i += 8;
    let bytes = &content[i..i + size as usize];
    i += size as usize;
    *pointer = i as u64;

    let mut compressed = Vec::new();
    {
        let mut encoder = Encoder::new(&mut compressed, 3)?;
        encoder.write_all(&bytes)?;
        encoder.finish()?;
    }

    let check = *blake3::hash(&compressed).as_bytes();

    if hash != check {
        return Err(Errors::HashMismatch);
    }

    ctx.save_snapshot(bytes.to_vec())?;
    Ok(())
}
