mod analysis;
mod catalog;
mod configuration;
mod execution;
mod expression;
mod failure;
mod limit;
mod lowering;
mod path;
mod request;
mod response;
mod rule;
mod selection;
mod shape;

use catalog::Catalog;
use execution::Execution;
use failure::{Code, Failure};
use photonic::prism::Verdict;
use photonic::runtime::Runtime;
use request::{Request, Text};
use serde::Serialize;
use spectrum::numbering::Numbering;
use wasm_bindgen::prelude::wasm_bindgen;

const VERSION: u32 = 3;

#[derive(Serialize)]
struct Lowering {
    program: lowering::Lowered,
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
    Ok(Lowering {
        program: lowering::Lowered::from(&program),
    })
}

// The canonical program runs, as Spectrum's explorations do, so that every configuration and event
// here takes the handle the command line and every question give it.
fn exploration(input: &str) -> Result<Exploration, Failure> {
    let query: Request = request::read(input)?;
    let program = query.program()?;
    let canonical = spectrum::order::exhaustive(&program);
    let target = query.target(&canonical.program, &canonical.naming)?;
    let symmetry = analysis::analysis(&program);
    let mut runtime = Runtime::new(&canonical.program);
    runtime.run(limit::EXPLORATION.work, limit::EXPLORATION.bound);
    let snapshot = runtime.snapshot();
    let numbering = Numbering::from(&snapshot);
    let definition = Catalog::new(&snapshot.definition, &program, &canonical.naming);
    Ok(Exploration {
        verdict: target
            .iter()
            .map(|goal| {
                let verdict = runtime.verdict(goal);
                Verdict {
                    witness: verdict
                        .witness
                        .map(|witness| numbering.configuration(witness)),
                    ..verdict
                }
            })
            .collect(),
        execution: Execution::new(snapshot, &numbering, &canonical.naming, definition),
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

#[wasm_bindgen]
pub fn select(input: &str) -> String {
    response::respond(selection::select(input))
}
