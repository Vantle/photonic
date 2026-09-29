use spectrum::failure::{Code, Failure};
use spectrum::subject::Reader;

pub struct Disk;

impl Reader for Disk {
    fn read(&self, path: &str) -> Result<String, Failure> {
        let failure = |error: std::io::Error| Failure::new(Code::File, format!("{path}: {error}"));
        if !std::fs::metadata(path).map_err(failure)?.is_file() {
            return Err(Failure::new(Code::File, format!("{path}: not a file")));
        }
        std::fs::read_to_string(path).map_err(failure)
    }
}
