mod analysis;
mod catalog;
mod configuration;
mod execution;
mod expression;
mod failure;
mod limit;
mod path;
mod request;
mod response;
mod shape;

use execution::Execution;
use failure::{Code, Failure};
use frontend::source::Program;
use photonic::prism::Verdict;
use photonic::runtime::Runtime;
use request::{Request, Text};
use serde::Serialize;
use wasm_bindgen::prelude::wasm_bindgen;

const VERSION: u32 = 3;

#[derive(Serialize)]
struct Lowering {
    program: Program,
}

#[derive(Serialize)]
struct Exploration {
    execution: Execution,
    verdict: Vec<Verdict>,
    #[serde(skip_serializing_if = "Option::is_none")]
    symmetry: Option<analysis::Analysis>,
}

fn lowering(input: &str) -> Result<Lowering, Failure> {
    let text: Text = request::read(input)?;
    let program = frontend::lowering::parse(&text.source)
        .map_err(|error| Failure::located(Code::Source, &error, &text.source))?;
    Ok(Lowering { program })
}

fn exploration(input: &str) -> Result<Exploration, Failure> {
    let query: Request = request::read(input)?;
    let program = query.program()?;
    let target = query.target(&program)?;
    let symmetry = analysis::analysis(&program);
    let mut runtime = Runtime::new(&program);
    runtime.run(limit::EXPLORATION.work, limit::EXPLORATION.bound);
    Ok(Exploration {
        verdict: target.iter().map(|goal| runtime.verdict(goal)).collect(),
        execution: Execution::from(runtime.snapshot()),
        symmetry,
    })
}

#[wasm_bindgen]
pub fn lower(input: &str) -> String {
    response::respond(lowering(input))
}

#[wasm_bindgen]
pub fn explore(input: &str) -> String {
    response::respond(exploration(input))
}

#[wasm_bindgen]
pub fn shape(input: &str) -> String {
    response::respond(shape::partition(input))
}
