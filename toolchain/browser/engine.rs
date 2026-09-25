mod analysis;
mod expression;
mod failure;
mod path;
mod request;
mod response;
mod shape;

use failure::{Code, Failure};
use photonic::prism::{Search, Verdict};
use photonic::runtime::Limit;
use photonic::snapshot::{Event, Node, Snapshot, View};
use photonic::source::Program;
use request::Request;
use serde::Serialize;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Serialize)]
struct Lowering {
    program: Program,
}

#[derive(Serialize)]
struct Transition {
    #[serde(flatten)]
    event: Event,
    deduction: Vec<usize>,
}

#[derive(Serialize)]
struct Exploration {
    execution: Snapshot<Vec<Node>, Vec<Transition>, Vec<View>>,
    verdict: Vec<Verdict>,
    #[serde(skip_serializing_if = "Option::is_none")]
    symmetry: Option<analysis::Analysis>,
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
    let deduction = |evidence: &[usize]| {
        if evidence
            .iter()
            .any(|&index| view[index].source == view[index].target)
        {
            return Vec::new();
        }
        let mut chain = Vec::new();
        let mut cursor = view[evidence[0]].origin;
        while let Some(origin) = cursor {
            chain.push(origin.event);
            cursor = view[origin.view].origin;
        }
        chain.reverse();
        chain
    };
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
                deduction: deduction(&event.evidence),
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
    let symmetry = analysis::analysis(&program);
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
    Ok(Exploration {
        execution,
        verdict,
        symmetry,
    })
}

#[wasm_bindgen]
pub fn lower(source: &str) -> String {
    response::respond(parse(source))
}

#[wasm_bindgen]
pub fn explore(input: &str) -> String {
    response::respond(exploration(input))
}

#[wasm_bindgen]
pub fn compare(input: &str) -> String {
    response::respond(shape::comparison(input))
}
