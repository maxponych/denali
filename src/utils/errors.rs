use std::path::PathBuf;

use hex::FromHexError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Errors {
    #[error("Snapshot with the name \"{0}\" does not exist")]
    SnapshotDoesNotExist(String),

    #[error("Failed to open stdin")]
    StdinFailed,

    #[error("Remote did not return anything")]
    NoStdout,

    #[error("Hashes did not match!")]
    HashMismatch,

    #[error("Remote with the name \"{0}\" is not found")]
    RemoteNotFound(String),

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

    #[error("Hex decoding failed")]
    Hex(#[from] FromHexError),

    #[error("I/O operation failed:\n {0}")]
    Io(#[from] std::io::Error),

    #[error("UUID failed:\n {0}")]
    Uuid(#[from] uuid::Error),

    #[error("JSON error:\n {0}")]
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
