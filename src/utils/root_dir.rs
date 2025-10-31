use std::{collections::HashMap, env, fs, path::PathBuf};

use crate::utils::{Errors, MainManifest};

pub fn denali_root() -> PathBuf {
    dirs::home_dir().unwrap().join(".denali")
    //env::current_dir().unwrap().join(".denali")
}

pub fn make_root_dir() -> Result<(), Errors> {
    fs::create_dir_all(denali_root().join("objects"))?;
    fs::create_dir_all(denali_root().join("snapshots").join("meta"))?;
    fs::create_dir_all(denali_root().join("snapshots").join("projects"))?;
    let manifest_file = denali_root().join("manifest.json");
    let manifest_obj: MainManifest = MainManifest {
        projects: HashMap::new(),
        templates: HashMap::new(),
    };
    let manifest_data = serde_json::to_vec_pretty(&manifest_obj)?;
    fs::write(manifest_file, manifest_data)?;
    Ok(())
}
