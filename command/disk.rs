use spectrum::failure::{Code, Failure};
use spectrum::subject::Reader;
use std::path::{Path, PathBuf};

// Program files as the file system holds them. The command line reads any path it names; the
// protocol server reads only paths that lead inside the directory it started in once every link is
// followed, so that a client cannot read the rest of the machine through it.
#[derive(Default)]
pub struct Disk {
    root: Option<PathBuf>,
}

impl Disk {
    pub fn within(root: &Path) -> std::io::Result<Self> {
        Ok(Self {
            root: Some(std::fs::canonicalize(root)?),
        })
    }

    fn place(&self, path: &str) -> Result<PathBuf, Failure> {
        let Some(root) = &self.root else {
            return Ok(PathBuf::from(path));
        };
        let real = std::fs::canonicalize(path)
            .map_err(|error| Failure::new(Code::File, format!("{path}: {error}")))?;
        if !real.starts_with(root) {
            return Err(Failure::new(
                Code::File,
                format!(
                    "{path}: outside {}, the directory the server started in; the server reads no file outside it",
                    root.display()
                ),
            ));
        }
        Ok(real)
    }
}

impl Reader for Disk {
    fn read(&self, path: &str) -> Result<String, Failure> {
        let failure = |error: std::io::Error| Failure::new(Code::File, format!("{path}: {error}"));
        let place = self.place(path)?;
        if !std::fs::metadata(&place).map_err(failure)?.is_file() {
            return Err(Failure::new(Code::File, format!("{path}: not a file")));
        }
        frontend::encoding::decode(path, std::fs::read(&place).map_err(failure)?)
            .map_err(|error| Failure::new(Code::File, error.to_string()))
    }
}
