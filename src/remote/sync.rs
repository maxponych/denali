use std::{
    collections::{HashMap, HashSet},
    io::{Read, Write},
    process::{Command, Stdio},
    str::FromStr,
    usize,
};

use chrono::{DateTime, Utc};
use uuid::Uuid;
use zstd::{Decoder, Encoder};

use crate::utils::{
    CellRef, Errors, MainManifest, ProjectManifest, ProjectRef, Snapshots, context::AppContext,
};

use super::{
    PackType,
    helpers::{pack_snapshot, pack_tree, unpack_object, unpack_snapshot},
};

pub fn remote_sync(ctx: &AppContext, project: String, remote: String) -> Result<(), Errors> {
    ctx.make_root_dir()?;
    let manifest = ctx.load_main_manifest()?;
    let remote = manifest
        .remotes
        .get(&remote)
        .ok_or(Errors::RemoteNotFound(remote))?;

    let url = &remote.host;
    let path = &remote.path;

    let project = if project != "all" {
        manifest
            .projects
            .get(&project)
            .ok_or(Errors::ProjectNotFound(project))?
            .manifest
            .clone()
    } else {
        project
    };

    let mut ssh = Command::new("ssh")
        .arg("-o")
        .arg("BatchMode=yes")
        .arg(url)
        .arg(format!(
            "denali --root {} remote manifest {}",
            path, project
        ))
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .spawn()?;

    let mut compressed = Vec::new();

    if let Some(stdout) = ssh.stdout.as_mut() {
        stdout.read_to_end(&mut compressed)?;
    } else {
        return Err(Errors::NoStdout);
    }

    let mut content = Vec::new();
    {
        let mut decoder = Decoder::new(&compressed[..])?;
        decoder.read_to_end(&mut content)?;
    }

    let mut pack_stage_one: Vec<u8> = Vec::new();
    let mut pack_stage_two: Vec<u8> = Vec::new();

    unpack_stage_one(
        ctx,
        &content,
        &mut pack_stage_one,
        &mut pack_stage_two,
        project,
    )?;

    let mut compressed_stage_one = Vec::new();
    {
        let mut encoder = Encoder::new(&mut compressed_stage_one, 3)?;
        encoder.write_all(&pack_stage_one)?;
        encoder.finish()?;
    }

    let mut ssh = Command::new("ssh")
        .arg("-o")
        .arg("BatchMode=yes")
        .arg(url)
        .arg(format!("denali --root {} remote send", path))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    let mut stdin = ssh.stdin.take().ok_or(Errors::StdinFailed)?;
    stdin.write_all(&compressed_stage_one)?;
    drop(stdin);

    let mut compressed = Vec::new();

    if let Some(stdout) = ssh.stdout.as_mut() {
        stdout.read_to_end(&mut compressed)?;
    } else {
        return Err(Errors::NoStdout);
    }

    let mut content = Vec::new();
    {
        let mut decoder = Decoder::new(&compressed[..])?;
        decoder.read_to_end(&mut content)?;
    }

    unpack_stage_two(ctx, &content)?;

    let mut compressed_stage_two = Vec::new();
    {
        let mut encoder = Encoder::new(&mut compressed_stage_two, 3)?;
        encoder.write_all(&pack_stage_two)?;
        encoder.finish()?;
    }

    let mut ssh = Command::new("ssh")
        .arg("-o")
        .arg("BatchMode=yes")
        .arg(url)
        .arg(format!("denali --root {} remote receive", path))
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .spawn()?;

    let mut stdin = ssh.stdin.take().ok_or(Errors::StdinFailed)?;
    stdin.write_all(&compressed_stage_two)?;
    drop(stdin);

    Ok(())
}

fn unpack_stage_two(ctx: &AppContext, content: &[u8]) -> Result<(), Errors> {
    let mut pointer: u64 = 0;
    while (pointer as usize) < content.len() {
        let mode = PackType::from_byte(content[pointer as usize]);
        pointer += 1;
        if let Some(mode) = mode {
            match mode {
                PackType::Object => unpack_object(ctx, content, &mut pointer)?,
                PackType::Snapshot => unpack_snapshot(ctx, content, &mut pointer)?,
                _ => {
                    break;
                }
            }
        } else {
            break;
        }
    }

    Ok(())
}

fn unpack_stage_one(
    ctx: &AppContext,
    output: &[u8],
    pack: &mut Vec<u8>,
    send: &mut Vec<u8>,
    project: String,
) -> Result<(), Errors> {
    let mut pointer: u64 = 0;
    let (deleted_projects, mut main_manifest) = diff_manifest(ctx, output, &mut pointer)?;
    let uuid_to_name: HashMap<String, String> = main_manifest
        .projects
        .iter()
        .map(|(name, proj)| (proj.manifest.clone(), name.clone()))
        .collect();
    let mut remote_new_projects: HashSet<String> = if project == "all" {
        uuid_to_name.keys().cloned().collect()
    } else {
        let mut temp = HashSet::new();
        temp.insert(project.clone());
        temp
    };
    let mut snapshots_to_send = Vec::new();
    while (pointer as usize) < output.len() {
        let mode = PackType::from_byte(output[pointer as usize]);
        pointer += 1;
        if let Some(mode) = mode {
            match mode {
                PackType::Project => {
                    let (uuid, manifest) = unpack_project(output, &mut pointer)?;
                    let uuid_str = uuid.to_string();

                    if !deleted_projects.contains(&uuid) {
                        if let Some(current_name) = uuid_to_name.get(&uuid_str) {
                            if let Some(proj_ref) = main_manifest.projects.get_mut(current_name) {
                                let local_proj_manifest =
                                    ctx.load_project_manifest(uuid_str.clone())?;
                                let (request, snapshots_send, manifest) =
                                    diff_project(&local_proj_manifest, &manifest)?;
                                ctx.write_project_manifest(uuid_str.clone(), &manifest)?;
                                pack.extend_from_slice(&request);
                                snapshots_to_send.extend_from_slice(&snapshots_send);

                                if let Some(latest) = newest_snapshot(&manifest.snapshots) {
                                    proj_ref.latest = latest.hash;
                                } else {
                                    proj_ref.latest = String::new();
                                }
                                proj_ref.cells =
                                    manifest
                                        .cells
                                        .iter()
                                        .filter_map(|(n, c)| {
                                            if !c.is_deleted { Some(n.clone()) } else { None }
                                        })
                                        .collect();

                                let bytes = serde_json::to_vec(&manifest)?;
                                let size = bytes.len() as u64;
                                let mode = PackType::Project.as_byte();
                                send.push(mode);
                                send.extend_from_slice(uuid.as_bytes());
                                send.extend_from_slice(&size.to_be_bytes());
                                send.extend_from_slice(&bytes);
                                remote_new_projects.remove(&uuid_str);
                            } else {
                                eprintln!("UUIDs do not match");
                                return Err(Errors::InternalError);
                            }
                        } else {
                            eprintln!("No such UUID: {}", uuid.to_string());
                            return Err(Errors::InternalError);
                        }
                    }
                }
                _ => {
                    break;
                }
            }
        } else {
            break;
        }
    }

    for uuid in remote_new_projects {
        pack_project(ctx, uuid, send, &mut snapshots_to_send)?;
    }

    let bytes = if project == "all" {
        serde_json::to_vec(&main_manifest.projects)?
    } else {
        let mut temp = HashMap::new();
        let name = uuid_to_name.get(&project).ok_or(Errors::InternalError)?;
        temp.insert(
            name,
            main_manifest
                .projects
                .get(name)
                .ok_or(Errors::InternalError)?,
        );
        serde_json::to_vec(&temp)?
    };

    let size = bytes.len() as u64;
    let mode = PackType::Main.as_byte();
    send.push(mode);
    send.extend_from_slice(&size.to_be_bytes());
    send.extend_from_slice(&bytes);
    ctx.write_main_manifest(&main_manifest)?;

    pack_snapshots(ctx, snapshots_to_send, send)?;
    Ok(())
}

fn diff_manifest(
    ctx: &AppContext,
    output: &[u8],
    pointer: &mut u64,
) -> Result<(HashSet<Uuid>, MainManifest), Errors> {
    let mut manifest = ctx.load_main_manifest()?;
    let mut deleted_uuids = HashSet::new();
    let mut i = *pointer as usize;
    let mode = PackType::from_byte(output[i]);
    i += 1;
    if let Some(mode) = mode {
        if mode.as_byte() == PackType::Main.as_byte() {
            let size = u64::from_be_bytes(output[i..i + 8].try_into()?);
            i += 8;
            let incoming_projects: HashMap<String, ProjectRef> =
                serde_json::from_slice(&output[i..i + size as usize])?;
            i += size as usize;

            let uuid_to_name: HashMap<String, String> = manifest
                .projects
                .iter()
                .map(|(name, r)| (r.manifest.clone(), name.clone()))
                .collect();

            for (incoming_name, incoming_ref) in incoming_projects {
                if let Some(local_name) = uuid_to_name.get(&incoming_ref.manifest) {
                    let local_ref = &manifest.projects[local_name];

                    if incoming_ref.timestamp > local_ref.timestamp {
                        if local_name != &incoming_name {
                            if manifest.projects.contains_key(&incoming_name) {
                                let name_1 = format!("{}-1", incoming_name);
                                let name_2 = format!("{}-2", incoming_name);

                                if let Some(existing) = manifest.projects.remove(&incoming_name) {
                                    manifest.projects.insert(name_1, existing);
                                }

                                manifest.projects.insert(name_2, incoming_ref.clone());
                            } else {
                                manifest
                                    .projects
                                    .insert(incoming_name.clone(), incoming_ref.clone());
                            }
                        }
                    }
                } else {
                    manifest
                        .projects
                        .entry(incoming_name.clone())
                        .and_modify(|existing_ref| {
                            if incoming_ref.timestamp > existing_ref.timestamp {
                                *existing_ref = incoming_ref.clone();
                            }
                        })
                        .or_insert(incoming_ref.clone());
                }

                if let Some(current_ref) = manifest.projects.get(&incoming_name) {
                    if current_ref.is_deleted {
                        if let Ok(id) = Uuid::from_str(&current_ref.manifest) {
                            deleted_uuids.insert(id);
                        }
                    }
                }
            }
        } else {
            return Ok((HashSet::new(), manifest));
        }
    } else {
        eprintln!("No mode byte");
        return Err(Errors::InternalError);
    }

    *pointer = i as u64;

    Ok((deleted_uuids, manifest))
}

fn diff_project(
    one: &ProjectManifest,
    two: &ProjectManifest,
) -> Result<(Vec<u8>, Vec<u8>, ProjectManifest), Errors> {
    let mut new_manifest = (*one).clone();

    if one.timestamp > two.timestamp {
        new_manifest.timestamp = one.timestamp;
        new_manifest.description = one.description.clone();
        new_manifest.name = one.name.clone();
    } else {
        new_manifest.timestamp = two.timestamp;
        new_manifest.description = two.description.clone();
        new_manifest.name = two.name.clone();
    }

    let (snapshots, mut fetch) = diff_snapshots(&one.snapshots, &two.snapshots);
    new_manifest.snapshots.extend(snapshots);
    let (cells, extend) = diff_cells(&one.cells, &two.cells);
    new_manifest.cells.extend(cells);
    fetch.extend_from_slice(&extend);

    let (_, mut send) = diff_snapshots(&two.snapshots, &one.snapshots);
    let (_, extend) = diff_cells(&two.cells, &one.cells);
    send.extend_from_slice(&extend);

    Ok((fetch, send, new_manifest))
}

fn unpack_project(content: &[u8], pointer: &mut u64) -> Result<(Uuid, ProjectManifest), Errors> {
    let mut i = *pointer as usize;

    let uuid = Uuid::from_bytes(content[i..i + 16].try_into()?);
    i += 16;

    let size = u64::from_be_bytes(content[i..i + 8].try_into()?);
    i += 8;

    let data = &content[i..i + size as usize];
    i += size as usize;

    *pointer = i as u64;

    let manifest: ProjectManifest = serde_json::from_slice(data)?;

    Ok((uuid, manifest))
}

fn pack_project(
    ctx: &AppContext,
    uuid: String,
    pack: &mut Vec<u8>,
    snapshots: &mut Vec<u8>,
) -> Result<(), Errors> {
    let manifest = ctx.load_project_manifest(uuid.clone())?;
    let bytes = serde_json::to_vec_pretty(&manifest)?;
    let size = (bytes.len() as u64).to_be_bytes();
    let uuid = Uuid::from_str(&uuid)?;
    pack.push(PackType::Project.as_byte());
    pack.extend_from_slice(uuid.as_bytes());
    pack.extend_from_slice(&size);
    pack.extend_from_slice(&bytes);

    for (_, snapshot) in manifest.snapshots {
        let mut hash = [0u8; 32];
        hex::decode_to_slice(snapshot.hash, &mut hash)?;
        snapshots.extend_from_slice(&hash);
    }

    for (_, cell_ref) in manifest.cells {
        for (_, snapshot) in cell_ref.snapshots {
            let mut hash = [0u8; 32];
            hex::decode_to_slice(snapshot.hash, &mut hash)?;
            snapshots.extend_from_slice(&hash);
        }
    }
    Ok(())
}

fn diff_snapshots(
    one: &HashMap<String, Snapshots>,
    two: &HashMap<String, Snapshots>,
) -> (HashMap<String, Snapshots>, Vec<u8>) {
    let mut needed = Vec::new();
    let hash_to_meta_one: HashMap<String, (String, Snapshots)> = one
        .iter()
        .map(|(k, v)| (v.hash.clone(), (k.clone(), v.clone())))
        .collect();

    let diff_snapshots: HashMap<String, Snapshots> = two
        .iter()
        .flat_map(|(k, v)| {
            let mut results = Vec::new();
            if let Some(snapshot_one) = one.get(k) {
                if v.hash == snapshot_one.hash {
                    if v.timestamp > snapshot_one.timestamp {
                        return results;
                    } else {
                        results.push((k.clone(), snapshot_one.clone()));
                    }
                } else {
                    if v.timestamp > snapshot_one.timestamp {
                        results.push((k.clone(), v.clone()));
                        results.push((
                            format!("{}-{}", k, snapshot_one.timestamp),
                            snapshot_one.clone(),
                        ));
                    } else {
                        results.push((k.clone(), snapshot_one.clone()));
                        results.push((format!("{}-{}", k, v.timestamp), v.clone()));
                    }
                }
            } else if let Some((old_name, snapshot_one)) = hash_to_meta_one.get(&v.hash) {
                if v.timestamp > snapshot_one.timestamp {
                    results.push((k.clone(), v.clone()));
                } else {
                    results.push((old_name.clone(), snapshot_one.clone()));
                }
            } else {
                if !v.is_deleted {
                    if let Ok(decoded) = hex::decode(&v.hash) {
                        needed.extend_from_slice(&decoded);
                    }
                }
                results.push((k.clone(), v.clone()));
            }

            results
        })
        .collect();

    (diff_snapshots, needed)
}

fn diff_cells(
    one: &HashMap<String, CellRef>,
    two: &HashMap<String, CellRef>,
) -> (HashMap<String, CellRef>, Vec<u8>) {
    let mut needed = Vec::new();
    let uuid_to_key_one: HashMap<String, (&String, &CellRef)> =
        one.iter().map(|(k, v)| (v.uuid.clone(), (k, v))).collect();
    let mut taken_names = HashSet::new();

    let diff_cells: HashMap<String, CellRef> = two
        .iter()
        .filter_map(|(k, v)| {
            if let Some((old_key, old_cell)) = uuid_to_key_one.get(&v.uuid) {
                let is_v_newer = v.timestamp > old_cell.timestamp;

                let mut winner = if is_v_newer {
                    v.clone()
                } else {
                    (*old_cell).clone()
                };
                let mut final_key = if is_v_newer {
                    k.clone()
                } else {
                    (*old_key).clone()
                };

                if taken_names.contains(&final_key) {
                    let mut counter = 1;
                    let base_name = final_key.clone();
                    while taken_names.contains(&final_key) {
                        final_key = format!("{}-{}", base_name, counter);
                        counter += 1;
                    }
                }

                if winner.description.is_empty() && !old_cell.description.is_empty() {
                    winner.description = old_cell.description.clone();
                }

                winner.timestamp = if is_v_newer {
                    v.timestamp
                } else {
                    old_cell.timestamp
                };

                winner.is_deleted = if is_v_newer {
                    v.is_deleted
                } else {
                    old_cell.is_deleted
                };

                if !winner.is_deleted {
                    let (snpapshots, pack) = diff_snapshots(&old_cell.snapshots, &v.snapshots);
                    needed.extend_from_slice(&pack);
                    winner.snapshots.extend(snpapshots);
                    if let Some(latest) = newest_snapshot(&winner.snapshots) {
                        winner.latest = latest.hash;
                    } else {
                        winner.latest = String::new();
                    }
                }

                taken_names.insert(final_key.clone());
                Some((final_key, winner))
            } else {
                if !v.is_deleted {
                    for (_, snapshot) in &v.snapshots {
                        if let Ok(hash) = hex::decode(snapshot.hash.clone()) {
                            needed.extend_from_slice(&hash);
                        }
                    }
                }
                let mut final_key = k.clone();

                if taken_names.contains(&final_key) {
                    let mut counter = 1;
                    let base_name = final_key.clone();
                    while taken_names.contains(&final_key) {
                        final_key = format!("{}-{}", base_name, counter);
                        counter += 1;
                    }
                }

                taken_names.insert(final_key.clone());
                Some((final_key, v.clone()))
            }
        })
        .collect();

    (diff_cells, needed)
}

fn newest_snapshot(snapshots: &HashMap<String, Snapshots>) -> Option<Snapshots> {
    let mut newest_timestamp: Option<DateTime<Utc>> = None;
    let mut snap_meta: Option<Snapshots> = None;

    for (_name, snapshot) in snapshots {
        if snapshot.is_deleted {
            continue;
        }
        match newest_timestamp {
            Some(current_newest) if snapshot.timestamp <= current_newest => continue,
            _ => {
                newest_timestamp = Some(snapshot.timestamp);
                snap_meta = Some(snapshot.clone());
            }
        }
    }

    snap_meta
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
