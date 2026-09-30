use crate::failure::{Code, Failure};
use frontend::source::{Library, Program};
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

fn json(file: &str) -> bool {
    std::path::Path::new(file)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
}

pub(crate) fn lower(file: &str, text: &str, code: Code) -> Result<Program, Failure> {
    if json(file) {
        return Program::read(text).map_err(|error| Failure::located(code, &error, file, text));
    }
    frontend::lowering::parse(text).map_err(|error| Failure::located(code, &error, file, text))
}

fn library(file: &str, text: &str) -> Result<Library, Failure> {
    let library = if json(file) {
        Library::read(text)
    } else {
        frontend::lowering::library(text)
    };
    library.map_err(|error| Failure::located(Code::Library, &error, file, text))
}

impl Subject {
    pub fn assemble(&self, reader: &dyn Reader) -> Result<Program, Failure> {
        if self.file.is_empty() && self.source.is_none() {
            return Err(Failure::new(
                Code::Request,
                "name the program with file or source",
            ));
        }
        let mut program = Program::default();
        for path in &self.library {
            program.declare(library(path, &reader.read(path)?)?);
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
