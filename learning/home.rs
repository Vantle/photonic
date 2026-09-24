use serde::Serialize;
use serde::de::DeserializeOwned;
use std::io::Write;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const POOL: &str = "pool.json";
pub const ARCHIVE: &str = "archive.json";
pub const CHECKPOINT: &str = "model.checkpoint";
pub const DISCOVERY: &str = "discovery.jsonl";
pub const PROGRESS: &str = "progress.jsonl";
pub const PROGRAM: &str = "program";
pub const CURRICULUM: &str = "curriculum.json";

#[derive(Debug, Error)]
pub enum Failure {
    #[error("could not access {path}: {source}")]
    Access {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("{path} is malformed: {source}")]
    Format {
        path: PathBuf,
        source: serde_json::Error,
    },
}

pub struct Home {
    path: PathBuf,
}

impl Home {
    pub fn open(path: PathBuf) -> Result<Self, Failure> {
        let program = path.join(PROGRAM);
        std::fs::create_dir_all(&program).map_err(|source| Failure::Access {
            path: program,
            source,
        })?;
        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn file(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }

    pub fn load<Value: DeserializeOwned>(&self, name: &str) -> Result<Option<Value>, Failure> {
        let path = self.file(name);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(source) => return Err(Failure::Access { path, source }),
        };
        serde_json::from_str(&text)
            .map(Some)
            .map_err(|source| Failure::Format { path, source })
    }

    pub fn save<Value: Serialize>(&self, name: &str, value: &Value) -> Result<(), Failure> {
        let text = serde_json::to_string(value).map_err(|source| Failure::Format {
            path: self.file(name),
            source,
        })?;
        self.write(name, &text)
    }

    pub fn write(&self, name: &str, text: &str) -> Result<(), Failure> {
        let path = self.file(name);
        let temporary = path.with_extension("partial");
        std::fs::write(&temporary, text)
            .and_then(|()| std::fs::rename(&temporary, &path))
            .map_err(|source| Failure::Access { path, source })
    }

    pub fn append<Value: Serialize>(&self, name: &str, value: &Value) -> Result<(), Failure> {
        let path = self.file(name);
        let line = serde_json::to_string(value).map_err(|source| Failure::Format {
            path: path.clone(),
            source,
        })?;
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .and_then(|mut file| writeln!(file, "{line}"))
            .map_err(|source| Failure::Access { path, source })
    }
}
