mod expression;
mod failure;
mod product;
mod response;
mod session;

use failure::{Code, Failure};
use photonic::prism::{Search, Verdict};
use photonic::runtime::Limit;
use photonic::snapshot::Snapshot;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    version: u32,
    source: String,
    #[serde(default)]
    targets: Vec<String>,
}

#[derive(Serialize)]
struct Execution {
    execution: Snapshot,
    verdict: Vec<Verdict>,
}

fn evaluate(input: &[u8]) -> Result<Execution, Failure> {
    let request: Request =
        serde_json::from_slice(input).map_err(|error| Failure::new(Code::Request, error))?;
    if request.version != 1 {
        return Err(Failure::new(Code::Version, "unsupported request version"));
    }
    if request.targets.len() > 16 {
        return Err(Failure::new(
            Code::Target,
            "at most 16 target configurations are supported",
        ));
    }
    let program = photonic::lowering::parse(&request.source)
        .map_err(|error| Failure::new(Code::Source, error))?;
    let target = request
        .targets
        .iter()
        .map(|source| {
            let target = photonic::lowering::parse(source)
                .map_err(|error| Failure::new(Code::Target, error))?;
            Ok(target)
        })
        .collect::<Result<Vec<_>, Failure>>()?;
    let mut search = Search::new(program, Default::default());
    search.run(
        20000,
        Some(Limit {
            state: 128,
            cell: 128,
            frame: 16,
            world: 16,
            record: 100000,
        }),
    );
    let execution = search.snapshot();
    let mut verdict = Vec::new();
    for target in target {
        search.target(target);
        verdict.push(search.verdict());
    }
    Ok(Execution { execution, verdict })
}

#[wasm_bindgen]
pub fn execute(input: &str) -> String {
    response::respond(if input.len() > 32768 {
        Err(Failure::new(Code::Size, "keep the request below 32 KiB"))
    } else {
        evaluate(input.as_bytes())
    })
}

#[wasm_bindgen]
pub fn calculate(input: &str) -> String {
    response::respond(expression::run(input))
}

#[wasm_bindgen]
pub fn multiply(left: u8, right: u8) -> String {
    response::respond(product::run(left, right))
}
