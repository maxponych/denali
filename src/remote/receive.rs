use std::{
    collections::HashMap,
    io::{Read, stdin},
};

use uuid::Uuid;
use zstd::Decoder;

use crate::utils::{Errors, ProjectManifest, ProjectRef, context::AppContext};

use super::{
    PackType,
    helpers::{unpack_object, unpack_snapshot},
};

pub fn remote_receive(ctx: &AppContext) -> Result<(), Errors> {
    let mut input = Vec::new();
    stdin().read_to_end(&mut input)?;

    let mut content = Vec::new();
    {
        let mut decoder = Decoder::new(&input[..])?;
        decoder.read_to_end(&mut content)?;
    }

    unpack(ctx, &content)?;

    Ok(())
}

fn unpack(ctx: &AppContext, content: &Vec<u8>) -> Result<(), Errors> {
    let mut pointer: u64 = 0;
    while (pointer as usize) < content.len() {
        let mode = PackType::from_byte(content[pointer as usize]);
        pointer += 1;
        if let Some(mode) = mode {
            match mode {
                PackType::Object => unpack_object(ctx, content, &mut pointer)?,
                PackType::Snapshot => unpack_snapshot(ctx, content, &mut pointer)?,
                PackType::Main => unpack_main(ctx, content, &mut pointer)?,
                PackType::Project => unpack_project(ctx, content, &mut pointer)?,
                _ => {}
            }
        } else {
            break;
        }
    }

    Ok(())
}

fn unpack_main(ctx: &AppContext, content: &Vec<u8>, pointer: &mut u64) -> Result<(), Errors> {
    let mut i = *pointer as usize;
    let mut main_manifest = ctx.load_main_manifest()?;
    let uuid_to_name: HashMap<String, String> = main_manifest
        .projects
        .iter()
        .map(|(name, proj)| (proj.manifest.clone(), name.clone()))
        .collect();

    let size = u64::from_be_bytes(content[i..i + 8].try_into()?);
    i += 8;
    let incoming_projects: HashMap<String, ProjectRef> =
        serde_json::from_slice(&content[i..i + size as usize])?;
    i += size as usize;

    for (name, proj_ref) in incoming_projects {
        if let Some(local_name) = uuid_to_name.get(&proj_ref.manifest) {
            if *local_name != name {
                main_manifest.projects.remove(local_name);
                main_manifest.projects.insert(name, proj_ref);
            } else {
                main_manifest.projects.insert(name, proj_ref);
            }
        } else {
            main_manifest.projects.insert(name, proj_ref);
        }
    }

    *pointer = i as u64;

    ctx.write_main_manifest(&main_manifest)?;

    Ok(())
}

fn unpack_project(ctx: &AppContext, content: &Vec<u8>, pointer: &mut u64) -> Result<(), Errors> {
    let mut i = *pointer as usize;

    let uuid = Uuid::from_bytes(content[i..i + 16].try_into()?);
    i += 16;

    let size = u64::from_be_bytes(content[i..i + 8].try_into()?);
    i += 8;

    let data = &content[i..i + size as usize];
    i += size as usize;

    *pointer = i as u64;

    let manifest: ProjectManifest = serde_json::from_slice(data)?;

    ctx.write_project_manifest(uuid.to_string(), &manifest)?;

    Ok(())
}
