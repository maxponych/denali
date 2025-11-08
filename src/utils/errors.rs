use chrono::Duration;
use std::{array::TryFromSliceError, fmt, path::PathBuf};

#[derive(Debug)]
pub enum Errors {
    AlreadyInitialised,
    ConfigExists(String),
    SameName(String),
    NotADir(PathBuf),
    DoesntExist(PathBuf),
    NotInitialised(PathBuf),
    ProjectNotFound(String),
    InvalidNameFormat(String),
    TooBigDate,
    SnapshotDoesntExist(String),
    DateTime(String),
    SnapshotExists(String),
    ParentPath(String),
    InternalError,
    Io(std::io::Error),
    Json(serde_json::Error),
    TomlSer(toml::ser::Error),
    TomlDe(toml::de::Error),
    GlobError(globset::Error),
    NoMatches,
    Dialoguer(dialoguer::Error),
    Stoped,
}

impl fmt::Display for Errors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Errors::AlreadyInitialised => write!(f, "Path is already initialised"),
            Errors::SameName(name) => {
                write!(f, "Project with the name \"{}\" already exists", name)
            }
            Errors::NotADir(path) => {
                write!(f, "Provided path \"{}\" is not a directory", path.display())
            }
            Errors::DoesntExist(path) => {
                write!(f, "Directory or file \"{}\" does not exist", path.display())
            }
            Errors::Io(error) => {
                write!(f, "I/O operation failed:\n{}", error)
            }
            Errors::Json(error) => {
                write!(f, "Failed to read JSON:\n{}", error)
            }
            Errors::TomlSer(error) => {
                write!(f, "Failed to serialise TOML:\n{}", error)
            }
            Errors::TomlDe(error) => {
                write!(f, "Failed to deserialise TOML:\n{}", error)
            }
            Errors::NotInitialised(path) => {
                write!(
                    f,
                    "Project or cell with the name \"{}\" is not initialised",
                    path.display()
                )
            }
            Errors::ProjectNotFound(name) => {
                write!(f, "Project with the name \"{}\" does not exist", name)
            }
            Errors::InvalidNameFormat(name) => {
                write!(f, "Invalid name format: {}", name)
            }
            Errors::ParentPath(path) => {
                write!(
                    f,
                    "Provided path \"{}\" is initialised as a project path",
                    path
                )
            }
            Errors::ConfigExists(path) => {
                write!(
                    f,
                    "File with the name reserved by config wile exist at path: {}",
                    path
                )
            }
            Errors::InternalError => {
                write!(f, "Internal error happend")
            }
            Errors::GlobError(error) => {
                write!(f, "Error occured in globset:\n{}", error)
            }
            Errors::SnapshotDoesntExist(name) => {
                write!(f, "Snapshot with the name \"{}\" does not exist", name)
            }
            Errors::SnapshotExists(name) => {
                write!(f, "Snapshot with the name \"{}\" already exists", name)
            }
            Errors::DateTime(time) => {
                write!(f, "Invalid date/time format: \"{}\"", time)
            }
            Errors::TooBigDate => {
                write!(f, "Too big date value")
            }
            Errors::NoMatches => {
                write!(f, "No matches were found")
            }
            Errors::Dialoguer(err) => {
                write!(f, "Dialog error happend \n{}", err)
            }
            Errors::Stoped => {
                write!(f, "The program was stoped")
            }
        }
    }
}

impl From<dialoguer::Error> for Errors {
    fn from(err: dialoguer::Error) -> Self {
        Errors::Dialoguer(err)
    }
}

impl From<TryFromSliceError> for Errors {
    fn from(_: TryFromSliceError) -> Self {
        Errors::InternalError
    }
}

impl From<std::io::Error> for Errors {
    fn from(err: std::io::Error) -> Self {
        Errors::Io(err)
    }
}

impl From<serde_json::Error> for Errors {
    fn from(err: serde_json::Error) -> Self {
        Errors::Json(err)
    }
}

impl From<toml::ser::Error> for Errors {
    fn from(err: toml::ser::Error) -> Self {
        Errors::TomlSer(err)
    }
}

impl From<toml::de::Error> for Errors {
    fn from(err: toml::de::Error) -> Self {
        Errors::TomlDe(err)
    }
}

impl From<globset::Error> for Errors {
    fn from(err: globset::Error) -> Self {
        Errors::GlobError(err)
    }
}
