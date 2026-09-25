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
use photonic::prism::{Search, Verdict};
use photonic::source::Program;
use request::{Request, Text};
use serde::Serialize;
use wasm_bindgen::prelude::wasm_bindgen;

const VERSION: u32 = 2;

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
    let program = photonic::lowering::parse(&text.source)
        .map_err(|error| Failure::located(Code::Source, &error, &text.source))?;
    Ok(Lowering { program })
}

fn exploration(input: &str) -> Result<Exploration, Failure> {
    let query: Request = request::read(input)?;
    let program = query.program()?;
    let target = query.target(&program)?;
    let symmetry = analysis::analysis(&program);
    let mut search = Search::new(program, Program::default());
    search.run(limit::EXPLORATION.work, Some(limit::EXPLORATION.bound));
    let execution = Execution::try_from(search.snapshot())?;
    let verdict = target
        .into_iter()
        .map(|goal| {
            search.target(goal);
            search.verdict()
        })
        .collect();
    Ok(Exploration {
        execution,
        verdict,
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
pub fn compare(input: &str) -> String {
    response::respond(shape::partition(input))
}
