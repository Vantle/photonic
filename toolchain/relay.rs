use std::path::{Path, PathBuf};
use std::process::{ExitCode, ExitStatus};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Failure {
    #[error("{0}")]
    Usage(&'static str),
    #[error("missing runfile: {0}")]
    Runfile(String),
    #[error("{path}: {source}")]
    Access {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("{path}: {message}")]
    Tool { path: PathBuf, message: String },
}

impl Failure {
    pub fn access(path: &Path) -> impl FnOnce(std::io::Error) -> Self + use<> {
        let path = path.to_path_buf();
        move |source| Self::Access { path, source }
    }

    pub fn report(self) -> ExitCode {
        eprintln!("{self}");
        ExitCode::FAILURE
    }
}

pub fn code(status: ExitStatus) -> ExitCode {
    ExitCode::from(
        status
            .code()
            .and_then(|code| u8::try_from(code).ok())
            .unwrap_or(1),
    )
}
