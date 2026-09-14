mod failure;

use failure::{Code, Failure};
use photonic::obsidian::Search;
use photonic::runtime::Limit;
use serde::Deserialize;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    version: u32,
    source: String,
    #[serde(default)]
    targets: Vec<String>,
}

fn evaluate(input: &[u8]) -> Result<serde_json::Value, Failure> {
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
            if !target.rule.is_empty() {
                return Err(Failure::new(
                    Code::Target,
                    "targets contain configurations without declarations",
                ));
            }
            Ok(target)
        })
        .collect::<Result<Vec<_>, Failure>>()?;
    let mut search = Search::new(program, target.first().cloned().unwrap_or_default())
        .map_err(|error| Failure::new(Code::Target, error))?;
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
    let execution = search.report().execution;
    let mut verdict = Vec::new();
    for target in target {
        search
            .target(target)
            .map_err(|error| Failure::new(Code::Target, error))?;
        verdict.push(search.verdict());
    }
    Ok(serde_json::json!({"version": 1, "execution": execution, "verdict": verdict}))
}

#[wasm_bindgen]
pub fn execute(input: &str) -> String {
    let result = if input.len() > 32768 {
        Err(Failure::new(Code::Size, "keep the request below 32 KiB"))
    } else {
        evaluate(input.as_bytes())
    };
    let response = match result {
        Ok(value) => value,
        Err(error) => serde_json::json!({"version": 1, "error": error}),
    };
    serde_json::to_string(&response).unwrap()
}
