mod expression;
mod failure;
mod path;
mod request;
mod response;

use failure::{Code, Failure};
use photonic::prism::{Search, Verdict};
use photonic::runtime::Limit;
use photonic::snapshot::{Event, Node, Snapshot, View};
use photonic::source::Program;
use request::Request;
use serde::Serialize;
use std::collections::HashSet;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Serialize)]
struct Lowering {
    program: Program,
}

#[derive(Serialize)]
struct Transition {
    #[serde(flatten)]
    event: Event,
    direct: bool,
}

#[derive(Serialize)]
struct Exploration {
    execution: Snapshot<Vec<Node>, Vec<Transition>, Vec<View>>,
    verdict: Vec<Verdict>,
}

const LIMIT: Limit = Limit {
    state: 128,
    cell: 128,
    frame: 16,
    world: 16,
    record: 100000,
};

fn parse(source: &str) -> Result<Lowering, Failure> {
    let program = photonic::lowering::parse(request::bound(source)?)
        .map_err(|error| Failure::located(Code::Source, &error, source))?;
    Ok(Lowering { program })
}

fn mark(snapshot: Snapshot) -> Snapshot<Vec<Node>, Vec<Transition>, Vec<View>> {
    let Snapshot {
        definition,
        closed,
        record,
        peak,
        queued,
        deferred,
        work,
        limit,
        state,
        event,
        view,
    } = snapshot;
    let identity = view
        .iter()
        .filter(|view| view.source == view.target)
        .map(|view| view.id)
        .collect::<HashSet<_>>();
    Snapshot {
        definition,
        closed,
        record,
        peak,
        queued,
        deferred,
        work,
        limit,
        state,
        event: event
            .into_iter()
            .map(|event| Transition {
                direct: event.evidence.iter().any(|view| identity.contains(view)),
                event,
            })
            .collect(),
        view: Vec::new(),
    }
}

fn exploration(input: &str) -> Result<Exploration, Failure> {
    let request = Request::read(input)?;
    let program = request.program()?;
    let target = request.target(&program)?;
    let mut search = Search::new(program, Program::default());
    search.run(20000, Some(LIMIT));
    let execution = mark(search.snapshot());
    let verdict = target
        .into_iter()
        .map(|target| {
            search.target(target);
            search.verdict()
        })
        .collect();
    Ok(Exploration { execution, verdict })
}

#[wasm_bindgen]
pub fn lower(source: &str) -> String {
    response::respond(parse(source))
}

#[wasm_bindgen]
pub fn explore(input: &str) -> String {
    response::respond(exploration(input))
}
