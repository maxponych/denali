use std::{
    collections::HashMap,
    env, fs,
    io::{Read, Write},
    path::PathBuf,
};

use dirs::home_dir;
use zstd::{Decoder, Encoder};

use super::{Errors, MainManifest, ProjectManifest, Snapshot};

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

    pub fn templates_path(&self) -> PathBuf {
        self.root.join("templates")
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
        fs::create_dir_all(path.join("templates"))?;
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

    pub fn load_main_manifest(&self) -> Result<MainManifest, Errors> {
        let data = fs::read(self.main_manifest_path())?;
        let manifest: MainManifest = serde_json::from_slice(&data)?;
        Ok(manifest)
    }

    pub fn load_project_manifest(&self, uuid: String) -> Result<ProjectManifest, Errors> {
        let data = fs::read(self.project_manifest_path(uuid))?;
        let manifest: ProjectManifest = serde_json::from_slice(&data)?;
        Ok(manifest)
    }

    pub fn write_main_manifest(&self, manifest: &MainManifest) -> Result<(), Errors> {
        let data = serde_json::to_vec_pretty(manifest)?;
        fs::write(self.main_manifest_path(), &data)?;
        Ok(())
    }

    pub fn write_project_manifest(
        &self,
        uuid: String,
        manifest: &ProjectManifest,
    ) -> Result<(), Errors> {
        let data = serde_json::to_vec_pretty(manifest)?;
        fs::write(self.project_manifest_path(uuid), &data)?;
        Ok(())
    }

    pub fn save_object(&self, content: Vec<u8>) -> Result<[u8; 32], Errors> {
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

        let obj_path = self.root.join("objects").join(dir);
        fs::create_dir_all(&obj_path)?;
        let file_path = obj_path.join(filename);

        fs::write(&file_path, &compressed)?;

        Ok(*hash.as_bytes())
    }

    pub fn save_snapshot(&self, content: Vec<u8>) -> Result<[u8; 32], Errors> {
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

        let obj_path = self.root.join("snapshots").join("meta").join(dir);
        fs::create_dir_all(&obj_path)?;
        let file_path = obj_path.join(filename);

        fs::write(&file_path, &compressed)?;

        Ok(*hash.as_bytes())
    }

    pub fn load_object(&self, hash: String) -> Result<Vec<u8>, Errors> {
        let file_path = self.objects_path().join(&hash[..3]).join(&hash[3..]);
        let data = fs::read(file_path)?;
        let mut content = Vec::new();
        {
            let mut decoder = Decoder::new(&data[..])?;
            decoder.read_to_end(&mut content)?;
        }
        Ok(content)
    }

    pub fn load_snapshot(&self, hash: String) -> Result<Snapshot, Errors> {
        let dir = &hash[..3];
        let filename = &hash[3..];

        let meta_path = self.snapshots_path().join(dir).join(filename);
        let meta_data_cmp = fs::read(meta_path)?;
        let mut meta_data = Vec::new();
        {
            let mut decoder = Decoder::new(&meta_data_cmp[..])?;
            decoder.read_to_end(&mut meta_data)?;
        }

        let meta: Snapshot = serde_json::from_slice(&meta_data)?;

        Ok(meta)
    }
}
