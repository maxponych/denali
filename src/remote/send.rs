use std::{
    collections::HashSet,
    io::{Read, Write, stdin, stdout},
};

use zstd::{Decoder, Encoder};

use crate::utils::{Errors, context::AppContext};

use super::helpers::{pack_snapshot, pack_tree};

pub fn remote_send(ctx: &AppContext) -> Result<(), Errors> {
    let mut input = Vec::new();
    stdin().read_to_end(&mut input)?;

    let mut content = Vec::new();
    {
        let mut decoder = Decoder::new(&input[..])?;
        decoder.read_to_end(&mut content)?;
    }

    let mut pack = Vec::new();
    pack_snapshots(ctx, content, &mut pack)?;

    let mut compressed = Vec::new();
    {
        let mut encoder = Encoder::new(&mut compressed, 3)?;
        encoder.write_all(&pack)?;
        encoder.finish()?;
    }

    stdout().write_all(&compressed)?;

    Ok(())
}

fn pack_snapshots(ctx: &AppContext, snapshots: Vec<u8>, send: &mut Vec<u8>) -> Result<(), Errors> {
    let mut i = 0;
    let mut copied = HashSet::new();
    while i < snapshots.len() {
        let hash: [u8; 32] = snapshots[i..i + 32].try_into()?;
        i += 32;
        let hash_str = hex::encode(hash);
        if !copied.contains(&hash_str) {
            pack_snapshot(ctx, &hash, send)?;
            copied.insert(hash_str.clone());
            let snapshot = ctx.load_snapshot(hash_str)?;
            if !copied.contains(&snapshot.root) {
                pack_tree(ctx, snapshot.root, send, &mut copied)?;
            }
        }
    }
    Ok(())
}
