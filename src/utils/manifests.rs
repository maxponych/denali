use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
pub struct Snapshots {
    pub hash: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MainManifest {
    pub projects: HashMap<String, ProjectRef>,
    pub templates: HashMap<String, TemplateRef>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TemplateRef {
    pub tree: String,
    pub config: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProjectRef {
    pub path: String,
    pub manifest: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub latest: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cells: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CellRef {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub latest: String,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub snapshots: HashMap<String, Snapshots>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProjectManifest {
    pub source: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    pub timestamp: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub snapshots: HashMap<String, Snapshots>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub cells: HashMap<String, CellRef>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
pub struct Snapshot {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    pub timestamp: DateTime<Utc>,
    pub root: String,
    pub permissions: [u8; 4],
}
