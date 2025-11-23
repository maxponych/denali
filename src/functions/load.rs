use std::{
    collections::HashMap,
    env, fs,
    io::Read,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    time::SystemTime,
};

use chrono::{
    DateTime, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc, offset::LocalResult,
};
use zstd::Decoder;

use crate::utils::{
    DenaliToml, Errors, MainManifest, ProjectConfig, ProjectManifest, ProjectRef, Snapshot,
    TreeStruct, context::AppContext, file_type::FileType, parse_name,
};

#[derive(Debug)]
pub struct Filter {
    pub before: Option<DateTime<Utc>>,
    pub after: Option<DateTime<Utc>>,
    pub name: Option<String>,
}

pub struct LocalSnapshot {
    pub name: String,
    pub timestamp: DateTime<Utc>,
}

impl Filter {
    pub fn new(
        before: Option<DateTime<Utc>>,
        after: Option<DateTime<Utc>>,
        name: Option<String>,
    ) -> Self {
        Self {
            before,
            after,
            name,
        }
    }

    pub fn is_valid(&self, snapshot: &LocalSnapshot) -> bool {
        if let Some(before) = self.before {
            if snapshot.timestamp >= before {
                return false;
            }
        }

        if let Some(after) = self.after {
            if snapshot.timestamp <= after {
                return false;
            }
        }

        if let Some(name) = &self.name {
            if snapshot.name != *name {
                return false;
            }
        }

        true
    }
}

fn parse_datetime(input: &str) -> Result<DateTime<Utc>, Errors> {
    let s = input.trim();

    if let Ok(duration) = humantime::parse_duration(s) {
        let now = SystemTime::now();
        let then = now.checked_sub(duration).ok_or(Errors::TooBigDate)?;
        return Ok(DateTime::<Utc>::from(then));
    }

    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Utc));
    }

    if let Ok(ndt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M")
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%d-%m-%Y %H:%M"))
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M"))
    {
        return Ok(Utc.from_utc_datetime(&ndt));
    }

    if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .or_else(|_| NaiveDate::parse_from_str(s, "%d-%m-%Y"))
    {
        if let Some(ndt) = date.and_hms_opt(0, 0, 0) {
            return Ok(Utc.from_utc_datetime(&ndt));
        } else {
            return Err(Errors::DateTime(input.to_string()));
        }
    }

    if let Ok(time) = NaiveTime::parse_from_str(s, "%H:%M") {
        let today = Local::now().date_naive();
        let naive = today.and_time(time);
        match Local.from_local_datetime(&naive) {
            LocalResult::Single(local_dt) => return Ok(local_dt.with_timezone(&Utc)),
            LocalResult::Ambiguous(first, _second) => return Ok(first.with_timezone(&Utc)),
            LocalResult::None => return Err(Errors::DateTime(input.to_string())),
        }
    }

    Err(Errors::DateTime(input.to_string()))
}

fn build_filter(
    cli_before: Option<DateTime<Utc>>,
    cli_after: Option<DateTime<Utc>>,
    cli_name: Option<String>,
    toml_before: Option<DateTime<Utc>>,
    toml_after: Option<DateTime<Utc>>,
    toml_lock: Option<String>,
) -> Result<Filter, Errors> {
    if let Some(lock) = toml_lock {
        if !lock.is_empty() {
            return Ok(Filter::new(None, None, Some(lock)));
        }
    }

    let before = match (cli_before, toml_before) {
        (Some(c), Some(t)) => Some(std::cmp::min(c, t)),
        (Some(c), None) => Some(c),
        (None, Some(t)) => Some(t),
        _ => None,
    };

    let after = match (cli_after, toml_after) {
        (Some(c), Some(t)) => Some(std::cmp::max(c, t)),
        (Some(c), None) => Some(c),
        (None, Some(t)) => Some(t),
        _ => None,
    };

    Ok(Filter::new(before, after, cli_name))
}

fn wipe_dir(path: &Path) -> Result<(), Errors> {
    fs::remove_dir_all(path).ok();
    fs::create_dir_all(path)?;
    Ok(())
}

fn wipe_cell(ctx: &AppContext, uuid: String, name: String) -> Result<(), Errors> {
    let manifest = ctx.load_project_manifest(uuid)?;
    let cell = manifest
        .cells
        .get(&name)
        .ok_or(Errors::CellNotFound(name))?;
    wipe_dir(&Path::new(&cell.path))?;
    Ok(())
}

fn wipe_project(ctx: &AppContext, uuid: String) -> Result<(), Errors> {
    let manifest = ctx.load_project_manifest(uuid)?;
    let config_path = Path::new(&manifest.source).join(".denali.toml");
    let data = fs::read(config_path.clone())?;
    wipe_dir(&Path::new(&manifest.source))?;
    for (_, cell_ref) in &manifest.cells {
        wipe_dir(&Path::new(&cell_ref.path))?;
    }
    fs::write(config_path, data)?;
    Ok(())
}

pub fn load(
    ctx: &AppContext,
    project: String,
    name: Option<String>,
    path: Option<&Path>,
    before: Option<String>,
    after: Option<String>,
    with_config: bool,
    wipe: bool,
) -> Result<(), Errors> {
    let (project_name, cell_name) = parse_name(project.clone())?;

    let manifest: MainManifest = ctx.load_main_manifest()?;

    let proj = manifest
        .projects
        .get(&project_name)
        .ok_or_else(|| Errors::NotInitialised(PathBuf::from(&project)))?;

    if wipe {
        if let Some(p) = path {
            let config_path = Path::new(p).join(".denali.toml");
            let data = fs::read(config_path.clone()).ok();
            wipe_dir(p)?;
            if let Some(d) = data {
                fs::write(config_path, d)?;
            }
        } else {
            if let Some(c) = cell_name.clone() {
                wipe_cell(ctx, proj.manifest.clone(), c)?;
            } else {
                wipe_project(ctx, proj.manifest.clone())?;
            }
        }
    }

    if let Some(cell) = &cell_name {
        if !proj.cells.contains(cell) {
            return Err(Errors::NotInitialised(PathBuf::from(cell)));
        }
    }

    let mut is_root_path = true;
    if let Some(_) = path {
        is_root_path = false;
    }

    let project_manifest: ProjectManifest = ctx.load_project_manifest(proj.manifest.clone())?;
    let config = get_project_config(&project_manifest)?;

    if let Some(cell) = cell_name {
        make_cell_load(
            ctx,
            &manifest,
            before,
            after,
            &project_manifest,
            project_name,
            cell,
            is_root_path,
            &config,
            name,
            path,
        )?;
    } else {
        make_project_load(
            ctx,
            before,
            after,
            is_root_path,
            name,
            &config,
            proj,
            &project_manifest,
            path,
            with_config,
        )?;
    }

    Ok(())
}

fn make_project_load(
    ctx: &AppContext,
    before: Option<String>,
    after: Option<String>,
    is_root_path: bool,
    name: Option<String>,
    config: &DenaliToml,
    proj: &ProjectRef,
    project_manifest: &ProjectManifest,
    path: Option<&Path>,
    with_config: bool,
) -> Result<(), Errors> {
    let (before_cmp, after_cmp) = match (before, after) {
        (Some(bef), Some(aft)) => (Some(parse_datetime(&bef)?), Some(parse_datetime(&aft)?)),
        (Some(bef), None) => (Some(parse_datetime(&bef)?), None),
        (None, Some(aft)) => (None, Some(parse_datetime(&aft)?)),
        _ => (None, None),
    };

    let (toml_bef, toml_aft) = {
        if is_root_path {
            let bef = config.root.snapshot_before.trim();
            let aft = config.root.snapshot_after.trim();

            let before = if bef.is_empty() {
                None
            } else {
                Some(parse_datetime(bef)?)
            };

            let after = if aft.is_empty() {
                None
            } else {
                Some(parse_datetime(aft)?)
            };

            (before, after)
        } else {
            (None, None)
        }
    };

    let filter = build_filter(
        before_cmp,
        after_cmp,
        name.clone(),
        toml_bef,
        toml_aft,
        None,
    )?;

    let mut locks: HashMap<String, Filter> = HashMap::new();

    if is_root_path {
        for cell in &proj.cells {
            let Some(cell_cfg) = config.cells.get(cell) else {
                continue;
            };

            locks.insert(
                cell.clone(),
                build_filter(
                    before_cmp,
                    after_cmp,
                    name.clone(),
                    toml_bef,
                    toml_aft,
                    Some(cell_cfg.lock.clone()),
                )?,
            );
        }
    } else {
        for cell in &proj.cells {
            locks.insert(
                cell.to_string(),
                build_filter(before_cmp, after_cmp, name.clone(), None, None, None)?,
            );
        }
    }

    load_project(ctx, &project_manifest, &filter, &locks, path, with_config)?;
    Ok(())
}

fn make_cell_load(
    ctx: &AppContext,
    manifest: &MainManifest,
    before: Option<String>,
    after: Option<String>,
    project_manifest: &ProjectManifest,
    project_name: String,
    cell: String,
    is_root_path: bool,
    config: &DenaliToml,
    name: Option<String>,
    path: Option<&Path>,
) -> Result<(), Errors> {
    if !manifest
        .projects
        .get(&project_name)
        .ok_or(Errors::InternalError)?
        .cells
        .contains(&cell.clone())
    {
        return Err(Errors::ProjectNotFound(cell));
    }

    let (before_cmp, after_cmp) = match (before, after) {
        (Some(bef), Some(aft)) => (Some(parse_datetime(&bef)?), Some(parse_datetime(&aft)?)),
        (Some(bef), None) => (Some(parse_datetime(&bef)?), None),
        (None, Some(aft)) => (None, Some(parse_datetime(&aft)?)),
        _ => (None, None),
    };

    let (toml_bef, toml_aft) = if is_root_path {
        if let Some(cell_cfg) = config.cells.get(&cell) {
            let bef = cell_cfg.snapshot_before.trim();
            let aft = cell_cfg.snapshot_after.trim();

            let before = if bef.is_empty() {
                None
            } else {
                Some(parse_datetime(bef)?)
            };

            let after = if aft.is_empty() {
                None
            } else {
                Some(parse_datetime(aft)?)
            };

            (before, after)
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    let cell_name = cell.clone();

    let filter = build_filter(
        before_cmp,
        after_cmp,
        name,
        toml_bef,
        toml_aft,
        if is_root_path {
            Some(
                config
                    .cells
                    .get(&cell_name)
                    .ok_or(Errors::InternalError)?
                    .lock
                    .clone(),
            )
        } else {
            None
        },
    )?;

    load_cell(ctx, &project_manifest, &filter, cell_name.to_string(), path)?;
    Ok(())
}

fn get_project_config(project_manifest: &ProjectManifest) -> Result<DenaliToml, Errors> {
    let config_path = Path::new(&project_manifest.source).join(".denali.toml");

    let config: DenaliToml;

    if config_path.exists() && !config_path.is_dir() {
        let config_data = fs::read_to_string(&config_path)?;
        config = toml::from_str(&config_data)?;
    } else {
        config = DenaliToml {
            root: ProjectConfig {
                name: String::new(),
                description: String::new(),
                ignore: Vec::new(),
                snapshot_before: String::new(),
                snapshot_after: String::new(),
            },
            cells: HashMap::new(),
        };
    }
    Ok(config)
}

fn load_cell(
    ctx: &AppContext,
    manifest: &ProjectManifest,
    filter: &Filter,
    cell: String,
    dest: Option<&Path>,
) -> Result<(), Errors> {
    let mut newest_timestamp: Option<DateTime<Utc>> = None;
    let mut snap_meta = String::new();

    for (name, snapshot) in &manifest
        .cells
        .get(&cell)
        .ok_or(Errors::InternalError)?
        .snapshots
    {
        let local_snap = LocalSnapshot {
            name: name.to_string(),
            timestamp: snapshot.timestamp,
        };

        if filter.is_valid(&local_snap) {
            match newest_timestamp {
                Some(current_newest) if snapshot.timestamp <= current_newest => continue,
                _ => {
                    newest_timestamp = Some(snapshot.timestamp);
                    snap_meta = snapshot.hash.clone();
                }
            }
        }
    }

    if snap_meta.is_empty() {
        return Err(Errors::NoMatches);
    }

    let meta_dir = &snap_meta[..3];
    let meta_file = &snap_meta[3..];
    let meta_path = ctx.snapshots_path().join(meta_dir).join(meta_file);
    let meta_data_cmp = fs::read(meta_path)?;
    let mut meta_data = Vec::new();
    {
        let mut decoder = Decoder::new(&meta_data_cmp[..])?;
        decoder.read_to_end(&mut meta_data)?;
    }

    let meta: Snapshot = serde_json::from_slice(&meta_data)?;

    restore_cell(ctx, meta.root, dest, manifest, cell, &meta.permissions)?;

    Ok(())
}

fn load_project(
    ctx: &AppContext,
    manifest: &ProjectManifest,
    filter: &Filter,
    locks: &HashMap<String, Filter>,
    dest: Option<&Path>,
    with_config: bool,
) -> Result<(), Errors> {
    let mut newest_timestamp: Option<DateTime<Utc>> = None;
    let mut snap_meta = String::new();

    for (name, snapshot) in &manifest.snapshots {
        let local_snap = LocalSnapshot {
            name: name.to_string(),
            timestamp: snapshot.timestamp,
        };

        if filter.is_valid(&local_snap) {
            match newest_timestamp {
                Some(current_newest) if snapshot.timestamp <= current_newest => continue,
                _ => {
                    newest_timestamp = Some(snapshot.timestamp);
                    snap_meta = snapshot.hash.clone();
                }
            }
        }
    }

    if snap_meta.is_empty() {
        return Err(Errors::NoMatches);
    }

    let meta_dir = &snap_meta[..3];
    let meta_file = &snap_meta[3..];
    let meta_path = ctx.snapshots_path().join(meta_dir).join(meta_file);

    let meta_data_cmp = fs::read(meta_path)?;
    let mut meta_data = Vec::new();
    {
        let mut decoder = Decoder::new(&meta_data_cmp[..])?;
        decoder.read_to_end(&mut meta_data)?;
    }

    let meta: Snapshot = serde_json::from_slice(&meta_data)?;

    let (destination, own_path) = match dest {
        Some(p) => (env::current_dir()?.join(p), false),
        None => (PathBuf::from(manifest.source.clone()), true),
    };

    if !destination.exists() {
        return Err(Errors::DoesntExist(destination));
    } else if !destination.is_dir() {
        return Err(Errors::NotADir(destination));
    }

    restore(ctx, meta.root, &destination, with_config, manifest)?;

    for (cell, lock) in locks {
        let cell_path = destination.join(cell);
        if !own_path && !cell_path.exists() {
            fs::create_dir(&cell_path)?;
        }

        load_cell(
            ctx,
            manifest,
            lock,
            cell.to_string(),
            if own_path { None } else { Some(&cell_path) },
        )?;
    }
    Ok(())
}

fn restore_file(
    ctx: &AppContext,
    hash: String,
    dest: &Path,
    with_config: bool,
    mode: &[u8; 4],
) -> Result<(), Errors> {
    let content = ctx.load_object(hash)?;

    let file_name = dest
        .file_name()
        .ok_or(Errors::InternalError)?
        .to_string_lossy();

    if file_name == ".denali.toml" && with_config {
        if dest.exists() {
            fs::remove_file(&dest)?;
        }
        fs::write(dest, &content)?;
    }

    if file_name != ".denali.toml" {
        if dest.exists() {
            fs::remove_file(&dest)?;
        }
        fs::write(dest, &content)?;
        let perms = u32::from_be_bytes(mode.clone()) & 0x0FFF;
        let mut permissions = fs::metadata(&dest)?.permissions();
        permissions.set_mode(perms);
        fs::set_permissions(&dest, permissions)?;
    }

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

fn restore_cell(
    ctx: &AppContext,
    hash: String,
    dest: Option<&Path>,
    manifest: &ProjectManifest,
    name: String,
    mode: &[u8; 4],
) -> Result<(), Errors> {
    let tree = ctx.load_object(hash)?;
    let entries = parse_tree(&tree)?;

    let destination = match dest {
        Some(p) => env::current_dir()?.join(p),
        None => PathBuf::from(
            manifest
                .cells
                .get(&name)
                .ok_or(Errors::InternalError)?
                .path
                .clone(),
        ),
    };

    if !destination.exists() {
        fs::create_dir(&destination)?;
    } else if !destination.is_dir() {
        return Err(Errors::NotADir(destination));
    }

    let perms = u32::from_be_bytes(mode.clone()) & 0x0FFF;
    let mut permissions = fs::metadata(&destination)?.permissions();
    permissions.set_mode(perms);
    fs::set_permissions(&destination, permissions)?;

    for entry in entries {
        let target = destination.join(entry.name.clone());
        let mode = u32::from_be_bytes(entry.mode);
        let filetype = FileType::from_mode(mode);

        match filetype {
            FileType::Directory => {
                if !target.exists() {
                    fs::create_dir(&target)?;
                }
                let perms = mode & 0x0FFF;
                let mut permissions = fs::metadata(&target)?.permissions();
                permissions.set_mode(perms);
                fs::set_permissions(&target, permissions)?;

                restore(ctx, hex::encode(entry.hash), &target, false, manifest)?;
            }
            FileType::Symlink => {
                let temp_path =
                    String::from_utf8_lossy(&ctx.load_object(hex::encode(entry.hash))?).to_string();
                let link = Path::new(&temp_path);
                std::os::unix::fs::symlink(&target, link)?;
            }
            FileType::Regular => {
                restore_file(ctx, hex::encode(entry.hash), &target, false, &entry.mode)?;
            }
            FileType::Cell => {
                maybe_restore_cell(
                    ctx,
                    hex::encode(entry.hash),
                    &target,
                    &entry.name,
                    manifest,
                    &entry.mode,
                )?;
            }
            _ => continue,
        }
    }

    Ok(())
}

fn maybe_restore_cell(
    ctx: &AppContext,
    hash: String,
    path: &Path,
    name: &str,
    project: &ProjectManifest,
    mode: &[u8; 4],
) -> Result<(), Errors> {
    if let Some(_) = project.cells.get(name) {
        return Ok(());
    } else {
        let snapshot = ctx.load_snapshot(hash)?;
        restore_cell(
            ctx,
            snapshot.root,
            Some(path),
            project,
            name.to_string(),
            mode,
        )?;
        return Ok(());
    }
}

fn restore(
    ctx: &AppContext,
    hash: String,
    dest: &Path,
    with_config: bool,
    project: &ProjectManifest,
) -> Result<(), Errors> {
    let tree = ctx.load_object(hash)?;

    let entries = parse_tree(&tree)?;

    for entry in entries {
        let target = dest.join(entry.name.clone());
        let mode = u32::from_be_bytes(entry.mode);
        let filetype = FileType::from_mode(mode);

        match filetype {
            FileType::Directory => {
                if !target.exists() {
                    fs::create_dir(&target)?;
                }
                let perms = mode & 0x0FFF;
                let mut permissions = fs::metadata(&target)?.permissions();
                permissions.set_mode(perms);
                fs::set_permissions(&target, permissions)?;

                restore(ctx, hex::encode(entry.hash), &target, with_config, project)?;
            }
            FileType::Symlink => {
                if target.exists() {
                    if target.is_dir() {
                        fs::remove_dir_all(&target)?;
                    } else {
                        fs::remove_file(&target)?;
                    }
                }
                let stored = ctx.load_object(hex::encode(entry.hash))?;
                let symlink_target = PathBuf::from(String::from_utf8_lossy(&stored).to_string());

                std::os::unix::fs::symlink(&symlink_target, &target)?;
            }
            FileType::Regular => {
                restore_file(
                    ctx,
                    hex::encode(entry.hash),
                    &target,
                    with_config,
                    &entry.mode,
                )?;
            }
            FileType::Cell => {
                maybe_restore_cell(
                    ctx,
                    hex::encode(entry.hash),
                    &target,
                    &entry.name,
                    project,
                    &entry.mode,
                )?;
            }
            _ => continue,
        }
    }

    Ok(())
}
