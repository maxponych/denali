use std::{
    collections::HashMap,
    io::{Write, stdout},
    str::FromStr,
};

use uuid::Uuid;
use zstd::Encoder;

use crate::utils::{Errors, context::AppContext};

use super::PackType;

pub fn remote_manifest(ctx: &AppContext, project: String) -> Result<(), Errors> {
    ctx.make_root_dir()?;
    let manifest = ctx.load_main_manifest()?;
    let mut pack: Vec<u8> = Vec::new();

    if project == "all" {
        let refs = manifest.projects.clone();
        let bytes = serde_json::to_vec(&refs)?;
        let size = bytes.len() as u64;
        pack.push(PackType::Main.as_byte());
        pack.extend_from_slice(&size.to_be_bytes());
        pack.extend_from_slice(&bytes);

        for (_name, proj_ref) in manifest.projects {
            let proj_manifest = ctx.load_project_manifest(proj_ref.manifest.clone())?;
            let data = serde_json::to_vec(&proj_manifest)?;
            let id = Uuid::from_str(&proj_ref.manifest)?;
            let size = data.len() as u64;

            pack.push(PackType::Project.as_byte());
            pack.extend_from_slice(id.as_bytes());
            pack.extend_from_slice(&size.to_be_bytes());
            pack.extend_from_slice(&data);
        }
    } else {
        if let Ok(proj_ref) = ctx.load_project_manifest(project.clone()) {
            let mut main = HashMap::new();
            let main_ref = manifest
                .projects
                .get(&proj_ref.name)
                .ok_or(Errors::InternalError)?;
            main.insert(proj_ref.name.clone(), main_ref);
            let bytes = serde_json::to_vec(&main)?;
            let size = bytes.len() as u64;
            pack.push(PackType::Main.as_byte());
            pack.extend_from_slice(&size.to_be_bytes());
            pack.extend_from_slice(&bytes);
            let data = serde_json::to_vec(&proj_ref)?;
            let id = Uuid::from_str(&project)?;
            let size = data.len() as u64;

            pack.push(PackType::Project.as_byte());
            pack.extend_from_slice(id.as_bytes());
            pack.extend_from_slice(&size.to_be_bytes());
            pack.extend_from_slice(&data);
        } else {
            pack.push(PackType::NotFound.as_byte());
        }
    }

    let mut compressed = Vec::new();
    {
        let mut encoder = Encoder::new(&mut compressed, 3)?;
        encoder.write_all(&pack)?;
        encoder.finish()?;
    }

    stdout().write_all(&compressed)?;

    Ok(())
}
