use crate::failure::{Code, Failure};
use crate::request::Request;
use crate::response;
use photonic::path::{Event, Search};
use photonic::prism::Outcome;
use photonic::runtime::Limit;
use photonic::snapshot::{Definition, Node};
use photonic::source::Program;
use serde::Serialize;
use wasm_bindgen::prelude::wasm_bindgen;

const BUDGET: usize = 1000000;

const LIMIT: Limit = Limit {
    state: 32768,
    cell: 8192,
    frame: 1024,
    world: 512,
    record: 2000000,
};

#[derive(Serialize)]
struct Progress<'path> {
    #[serde(skip_serializing_if = "Option::is_none")]
    outcome: Option<Outcome>,
    state: Node,
    work: usize,
    event: usize,
    source: &'path str,
    definition: Vec<Definition>,
}

#[derive(Serialize)]
struct Inspection<'search> {
    event: &'search Event,
    before: Option<Node>,
    after: Option<Node>,
}

struct Session {
    search: Search,
    source: String,
    target: bool,
}

impl Session {
    fn follow(input: &str) -> Result<Self, Failure> {
        let request = Request::read(input)?;
        let program = request.program()?;
        let mut target = request.target(&program)?;
        if target.len() > 1 {
            return Err(Failure::new(
                Code::Target,
                "a path follows at most one target",
            ));
        }
        let goal = target.pop();
        Ok(Self {
            target: goal.is_some(),
            search: Search::new(program, goal.unwrap_or_default()),
            source: request.source().to_owned(),
        })
    }

    fn evaluate(input: &str) -> Result<Self, Failure> {
        let (program, source) = crate::expression::prepare(input)?;
        Ok(Self {
            search: Search::new(program, Program::default()),
            source,
            target: false,
        })
    }

    fn run(&mut self) -> Progress<'_> {
        self.search.run(BUDGET, LIMIT);
        let summary = self.search.summary();
        Progress {
            outcome: self.target.then_some(summary.outcome),
            state: self.search.current(),
            work: summary.work,
            event: summary.event,
            source: &self.source,
            definition: self.search.definition(),
        }
    }

    fn inspect(&self, index: usize) -> Result<Inspection<'_>, Failure> {
        self.search
            .transition(index)
            .map(|event| Inspection {
                event,
                before: self.search.inspect(event.source),
                after: self.search.inspect(event.target),
            })
            .ok_or_else(|| Failure::new(Code::Request, "No such transition."))
    }
}

#[wasm_bindgen]
pub struct Path {
    session: Result<Session, Failure>,
}

#[wasm_bindgen]
impl Path {
    #[wasm_bindgen(constructor)]
    pub fn new(input: &str) -> Self {
        Self {
            session: Session::follow(input),
        }
    }

    pub fn expression(input: &str) -> Self {
        Self {
            session: Session::evaluate(input),
        }
    }

    pub fn run(&mut self) -> String {
        match &mut self.session {
            Ok(session) => response::encode(session.run()),
            Err(failure) => response::reject(failure),
        }
    }

    pub fn inspect(&self, index: usize) -> String {
        match &self.session {
            Ok(session) => response::respond(session.inspect(index)),
            Err(failure) => response::reject(failure),
        }
    }
}
