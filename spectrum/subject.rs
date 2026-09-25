use crate::failure::{Code, Failure};
use photonic::source::Program;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, JsonSchema, PartialEq, Serialize)]
#[schemars(
    description = "A Photonic program: files and inline source, with libraries of declarations loaded first."
)]
pub struct Subject {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(
        description = "Program files in the order they are read: .wave or .particle source, or .json programs assembled by Bazel."
    )]
    pub file: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Photonic source written inline, read after the files.")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(
        description = "Library files holding declarations only, loaded before the program."
    )]
    pub library: Vec<String>,
}

pub trait Reader {
    fn read(&self, path: &str) -> Result<String, Failure>;
}

pub(crate) fn lower(file: &str, text: &str, code: Code) -> Result<Program, Failure> {
    if file.ends_with(".json") {
        return serde_json::from_str(text).map_err(|error| {
            Failure::new(code, format!("{file}: not an assembled program: {error}"))
        });
    }
    photonic::lowering::parse(text).map_err(|error| Failure::located(code, &error, file, text))
}

impl Subject {
    pub(crate) fn assemble(&self, reader: &dyn Reader) -> Result<Program, Failure> {
        if self.file.is_empty() && self.source.is_none() {
            return Err(Failure::new(
                Code::Request,
                "name the program with file or source",
            ));
        }
        let mut program = Program::default();
        for path in &self.library {
            let library = lower(path, &reader.read(path)?, Code::Library)?;
            program
                .declare(library, path)
                .map_err(|error| Failure::new(Code::Library, error.to_string()))?;
        }
        for path in &self.file {
            program.append(lower(path, &reader.read(path)?, Code::Source)?);
        }
        if let Some(source) = &self.source {
            program.append(lower("source", source, Code::Source)?);
        }
        Ok(program)
    }
}
