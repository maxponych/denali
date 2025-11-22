use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ProjectConfig {
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ignore: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub snapshot_before: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub snapshot_after: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CellConfig {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ignore: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub lock: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub snapshot_before: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub snapshot_after: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DenaliToml {
    pub root: ProjectConfig,
    #[serde(flatten)]
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub cells: HashMap<String, CellConfig>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TmplToml {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub placeholders: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub commands: Vec<String>,
}
