use crate::failure::{Code, Failure, Item};
use frontend::source::Program;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

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
pub struct Text {
    pub source: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    source: String,
    #[serde(default)]
    library: Vec<Library>,
    #[serde(default)]
    target: Vec<String>,
    #[serde(default)]
    preserve: bool,
}

fn bound(input: &str) -> Result<&str, Failure> {
    if input.len() > SIZE {
        return Err(Failure::new(Code::Size, "Keep the request below 128 KiB."));
    }
    Ok(input)
}

pub fn read<Body: DeserializeOwned>(input: &str) -> Result<Body, Failure> {
    decode(bound(input)?)
}

pub fn decode<Body: DeserializeOwned>(input: &str) -> Result<Body, Failure> {
    let mut body: Map<String, Value> =
        serde_json::from_str(input).map_err(|error| Failure::new(Code::Request, error))?;
    if body.remove("version") != Some(Value::from(crate::VERSION)) {
        return Err(Failure::new(
            Code::Version,
            format!(
                "Reload the page: this engine answers version {} requests.",
                crate::VERSION
            ),
        ));
    }
    serde_json::from_value(Value::Object(body)).map_err(|error| Failure::new(Code::Request, error))
}

impl Request {
    pub fn program(&self) -> Result<Program, Failure> {
        let mut program = Program::default();
        for library in &self.library {
            let declaration = frontend::lowering::parse(&library.source).map_err(|error| {
                Failure::new(Code::Library, format!("{}: {error}", library.name))
            })?;
            program
                .declare(declaration, &library.name)
                .map_err(|error| Failure::new(Code::Library, error))?;
        }
        let source = frontend::lowering::parse(&self.source)
            .map_err(|error| Failure::located(Code::Source, &error, &self.source))?;
        program.append(source);
        Ok(program)
    }

    pub fn target(&self, program: &Program) -> Result<Vec<Program>, Failure> {
        if self.target.len() > TARGET {
            return Err(Failure::new(
                Code::Target,
                "Use at most 16 target configurations.",
            ));
        }
        self.target
            .iter()
            .enumerate()
            .map(|(index, source)| {
                let mut target = frontend::lowering::parse(source).map_err(|error| {
                    Failure::located(Code::Target, &error, source).within(Item::Target(index))
                })?;
                if self.preserve {
                    target.preserve(program);
                }
                Ok(target)
            })
            .collect()
    }
}
