use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Errors {
    #[error("Command failed \"{0}\"")]
    CommandFailed(String),

    #[error("Template with the name \"{0}\" does not exist")]
    TemplateDoesntExist(String),

    #[error("Template with the name \"{0}\" already exists")]
    TemplateExists(String),

    #[error("Path is already initialised")]
    AlreadyInitialised,

    #[error("Config file already exists at: {0}")]
    ConfigExists(String),

    #[error("Project with the name \"{0}\" already exists")]
    SameName(String),

    #[error("Provided path \"{0}\" is not a directory")]
    NotADir(PathBuf),

    #[error("Directory or file \"{0}\" does not exist")]
    DoesntExist(PathBuf),

    #[error("Project or cell with the name \"{0}\" is not initialised")]
    NotInitialised(PathBuf),

    #[error("Project with the name \"{0}\" does not exist")]
    ProjectNotFound(String),

    #[error("Cell with the name \"{0}\" does not exist")]
    CellNotFound(String),

    #[error("Invalid name format: {0}")]
    InvalidNameFormat(String),

    #[error("Date is too large")]
    TooBigDate,

    #[error("Snapshot \"{0}\" already exists")]
    SnapshotExists(String),

    #[error("Invalid date/time format: {0}")]
    DateTime(String),

    #[error("Provided path \"{0}\" is inside an existing project path")]
    ParentPath(String),

    #[error("No matches found")]
    NoMatches,

    #[error("Stopped by user")]
    Stopped,

    #[error("I/O operation failed")]
    Io(#[from] std::io::Error),

    #[error("JSON error")]
    Json(#[from] serde_json::Error),

    #[error("TOML serialisation error")]
    TomlSer(#[from] toml::ser::Error),

    #[error("TOML deserialisation error")]
    TomlDe(#[from] toml::de::Error),

    #[error("Globset error")]
    GlobError(#[from] globset::Error),

    #[error("Dialog error")]
    Dialoguer(#[from] dialoguer::Error),

    #[error("Internal error")]
    InternalError,

    #[error("Can not find a home directory")]
    HomeNotFound,

    #[error("Failed to convert \"{0}\"")]
    TryFromSlice(#[from] std::array::TryFromSliceError),
}
