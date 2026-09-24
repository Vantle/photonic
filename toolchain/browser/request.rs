use crate::failure::{Code, Failure};
use photonic::source::Program;
use serde::Deserialize;

const SIZE: usize = 131072;
const TARGET: usize = 16;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Library {
    name: String,
    source: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    version: u32,
    source: String,
    #[serde(default)]
    library: Vec<Library>,
    #[serde(default)]
    target: Vec<String>,
    #[serde(default)]
    preserve: bool,
}

pub fn bound(input: &str) -> Result<&str, Failure> {
    if input.len() > SIZE {
        return Err(Failure::new(Code::Size, "keep the request below 128 KiB"));
    }
    Ok(input)
}

impl Request {
    pub fn read(input: &str) -> Result<Self, Failure> {
        let request: Self = serde_json::from_str(bound(input)?)
            .map_err(|error| Failure::new(Code::Request, error))?;
        if request.version != 1 {
            return Err(Failure::new(Code::Version, "unsupported request version"));
        }
        if request.target.len() > TARGET {
            return Err(Failure::new(
                Code::Target,
                "at most 16 target configurations are supported",
            ));
        }
        Ok(request)
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn program(&self) -> Result<Program, Failure> {
        let mut program = Program::default();
        for library in &self.library {
            let declaration = photonic::lowering::parse(&library.source).map_err(|error| {
                Failure::new(Code::Library, format!("{}: {error}", library.name))
            })?;
            program
                .declare(declaration, &library.name)
                .map_err(|error| Failure::new(Code::Library, error))?;
        }
        let source = photonic::lowering::parse(&self.source)
            .map_err(|error| Failure::located(Code::Source, &error, &self.source))?;
        program.append(source);
        Ok(program)
    }

    pub fn target(&self, program: &Program) -> Result<Vec<Program>, Failure> {
        self.target
            .iter()
            .map(|source| {
                let mut target = photonic::lowering::parse(source)
                    .map_err(|error| Failure::located(Code::Target, &error, source))?;
                if self.preserve {
                    target.preserve(program);
                }
                Ok(target)
            })
            .collect()
    }
}
