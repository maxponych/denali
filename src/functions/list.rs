use colored::*;
use std::collections::HashMap;

use crate::utils::context::AppContext;
use crate::utils::{CellRef, Errors, MainManifest, ProjectManifest, ProjectRef, Snapshots};

pub fn list(ctx: &AppContext, name: String) -> Result<(), Errors> {
    let (cell, project_name) = parse_name(&name)?;
    let manifest = ctx.load_main_manifest()?;
    if project_name == "all" && cell.is_none() {
        return print_all_projects(ctx, &manifest);
    }
    let proj_ref = manifest
        .projects
        .get(&project_name)
        .ok_or(Errors::InternalError)?;
    let proj_manifest = ctx.load_project_manifest(proj_ref.manifest.clone())?;
    if let Some(cell_name) = cell {
        let cell_ref = proj_manifest
            .cells
            .get(&cell_name)
            .ok_or(Errors::InternalError)?;
        print_cell_tree("└─", " ", cell_ref, &cell_name)?;
    } else {
        print_project_tree(&project_name, proj_ref, &proj_manifest)?;
    }
    Ok(())
}

fn print_project_tree(
    name: &str,
    proj_ref: &ProjectRef,
    proj_manifest: &ProjectManifest,
) -> Result<(), Errors> {
    let latest = latest_snapshot_name(&proj_manifest.snapshots, &proj_ref.latest);
    println!(
        "{} (latest: {}) - {}",
        name.cyan().bold(),
        latest.green(),
        proj_manifest.description.dimmed()
    );

    let mut snap_items: Vec<(&str, &Snapshots)> = proj_manifest
        .snapshots
        .iter()
        .map(|(n, s)| (n.as_str(), s))
        .collect();
    snap_items.sort_by(|a, b| b.1.timestamp.cmp(&a.1.timestamp));

    let mut cell_items: Vec<(&String, &CellRef)> = proj_manifest.cells.iter().collect();
    cell_items.sort_by_key(|(n, _)| *n);

    let total = snap_items.len() + cell_items.len();
    let mut idx = 0usize;

    for &(snap_name, snap) in &snap_items {
        idx += 1;
        let (branch, _) = branch_cont(idx == total);
        println!(
            " {}{} ({})",
            branch,
            snap_name,
            format_timestamp(&snap.timestamp.to_string()).dimmed()
        );
    }

    for &(cell_name, cell_ref) in &cell_items {
        idx += 1;
        let is_last = idx == total;
        let branch = if is_last { "└─" } else { "├─" };
        let cont = cont_for(is_last);
        print_cell_tree(branch, cont, cell_ref, cell_name)?;
    }
    Ok(())
}

fn print_cell_tree(
    branch: &str,
    cont: &str,
    cell_ref: &CellRef,
    cell_name: &str,
) -> Result<(), Errors> {
    let latest = latest_snapshot_name(&cell_ref.snapshots, &cell_ref.latest);
    println!(
        " {}{}{} (latest: {}) - {}",
        branch,
        " ",
        cell_name.yellow().bold(),
        latest.green(),
        cell_ref.description.dimmed()
    );

    let mut items: Vec<(&str, &Snapshots)> = cell_ref
        .snapshots
        .iter()
        .map(|(n, s)| (n.as_str(), s))
        .collect();
    items.sort_by(|a, b| b.1.timestamp.cmp(&a.1.timestamp));

    let cnt = items.len();
    for (i, &(snap_name, snap)) in items.iter().enumerate() {
        let is_last = i + 1 == cnt;
        let (snap_branch, _snap_cont) = branch_cont(is_last);
        println!(
            " {} {}{} ({})",
            cont,
            snap_branch,
            snap_name,
            format_timestamp(&snap.timestamp.to_string()).dimmed()
        );
    }
    Ok(())
}

fn branch_cont(is_last: bool) -> (&'static str, &'static str) {
    if is_last {
        ("└─ ", "   ")
    } else {
        ("├─ ", "│  ")
    }
}

fn cont_for(is_last: bool) -> &'static str {
    if is_last { "   " } else { "│  " }
}

fn parse_name(name: &str) -> Result<(Option<String>, String), Errors> {
    let mut parts = name.split('@');
    let cell_opt = parts.next().map(|s| s.to_string());
    let proj_opt = parts.next().map(|s| s.to_string());

    match (cell_opt, proj_opt) {
        (Some(cell), Some(proj)) if !cell.is_empty() => Ok((Some(cell), proj)),
        (Some(proj), None) => Ok((None, proj)),
        _ => Err(Errors::InvalidNameFormat(name.to_string())),
    }
}

fn latest_snapshot_name<'a>(
    snapshots: &'a HashMap<String, Snapshots>,
    latest_hash: &str,
) -> &'a str {
    snapshots
        .iter()
        .find(|(_, s)| s.hash == latest_hash)
        .map(|(name, _)| name.as_str())
        .unwrap_or("")
}

fn format_timestamp(ts: &str) -> String {
    ts.split('.').next().unwrap_or(ts).to_string()
}

fn print_all_projects(ctx: &AppContext, manifest: &MainManifest) -> Result<(), Errors> {
    let mut projects: Vec<_> = manifest.projects.iter().collect();
    projects.sort_by_key(|(name, _)| *name);

    for (i, (name, proj_ref)) in projects.into_iter().enumerate() {
        let proj_manifest = ctx.load_project_manifest(proj_ref.manifest.clone())?;
        print_project_tree(name, proj_ref, &proj_manifest)?;
        if i + 1 < manifest.projects.len() {
            println!();
        }
    }
    Ok(())
}
