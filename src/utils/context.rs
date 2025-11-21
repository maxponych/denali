use std::{collections::HashMap, env, fs, path::PathBuf};

use dirs::home_dir;

use super::{Errors, MainManifest};

pub struct AppContext {
    pub root: PathBuf,
}

impl AppContext {
    pub fn new(from: Option<PathBuf>) -> Result<Self, Errors> {
        let base = match from {
            Some(p) => {
                if p.is_absolute() {
                    p
                } else {
                    env::current_dir()?.join(p)
                }
            }
            None => home_dir().ok_or(Errors::HomeNotFound)?,
        };

        let root = base.join(".denali");

        fs::create_dir_all(&root)?;

        let root = root.canonicalize()?;

        Ok(Self { root })
    }

    pub fn main_manifest_path(&self) -> PathBuf {
        self.root.join("manifest.json")
    }

    pub fn project_manifest_path(&self, uuid: String) -> PathBuf {
        self.root
            .join("snapshots")
            .join("projects")
            .join(format!("{}.json", uuid))
    }

    pub fn snapshots_path(&self) -> PathBuf {
        self.root.join("snapshots").join("meta")
    }

    pub fn objects_path(&self) -> PathBuf {
        self.root.join("objects")
    }

    pub fn make_root_dir(&self) -> Result<(), Errors> {
        let path = self.root.clone();

        if !path.exists() {
            fs::create_dir_all(&path)?;
        }

        fs::create_dir_all(path.join("objects"))?;
        fs::create_dir_all(path.join("snapshots").join("meta"))?;
        fs::create_dir_all(path.join("snapshots").join("projects"))?;
        let manifest_file = path.join("manifest.json");
        if !manifest_file.exists() {
            let manifest_obj: MainManifest = MainManifest {
                projects: HashMap::new(),
                templates: HashMap::new(),
            };
            let manifest_data = serde_json::to_vec_pretty(&manifest_obj)?;
            fs::write(manifest_file, manifest_data)?;
        };
        Ok(())
    }
}
