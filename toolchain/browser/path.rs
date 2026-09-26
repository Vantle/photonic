use crate::catalog::Catalog;
use crate::configuration::Configuration;
use crate::failure::{Code, Failure};
use crate::limit;
use crate::request::{self, Request};
use crate::response;
use photonic::path::{Event, Search};
use photonic::place::Place;
use photonic::prism::Outcome;
use serde::Serialize;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Serialize)]
struct Evaluation<'path> {
    state: Configuration,
    source: &'path str,
}

#[derive(Serialize)]
struct Progress<'path> {
    #[serde(skip_serializing_if = "Option::is_none")]
    outcome: Option<Outcome>,
    work: usize,
    event: usize,
    definition: &'path Catalog,
    #[serde(flatten)]
    evaluation: Option<Evaluation<'path>>,
}

#[derive(Serialize)]
struct Step<'search> {
    source: usize,
    target: usize,
    rule: &'search str,
    footprint: &'search [Place],
    exact: &'search [Place],
}

impl<'search> Step<'search> {
    fn new(event: &'search Event, definition: &'search Catalog) -> Self {
        Self {
            source: event.source,
            target: event.target,
            rule: definition.name(event.rule),
            footprint: &event.footprint,
            exact: &event.exact,
        }
    }
}

#[derive(Serialize)]
struct Inspection<'search> {
    event: Step<'search>,
    before: Option<Configuration>,
    after: Option<Configuration>,
}

enum Mode {
    Follow { target: bool },
    Evaluate { source: String },
}

struct Session {
    search: Search,
    definition: Catalog,
    mode: Mode,
}

impl Session {
    fn new(search: Search, mode: Mode) -> Self {
        Self {
            definition: Catalog::from(search.definition()),
            search,
            mode,
        }
    }

    fn follow(input: &str) -> Result<Self, Failure> {
        let query: Request = request::read(input)?;
        let program = query.program()?;
        let mut target = query.target(&program)?;
        if target.len() > 1 {
            return Err(Failure::new(
                Code::Target,
                "A path follows at most one target.",
            ));
        }
        let goal = target.pop();
        let mode = Mode::Follow {
            target: goal.is_some(),
        };
        Ok(Self::new(Search::new(program, goal), mode))
    }

    fn evaluate(input: &str) -> Result<Self, Failure> {
        let (program, source) = crate::expression::prepare(input)?;
        Ok(Self::new(
            Search::new(program, None),
            Mode::Evaluate { source },
        ))
    }

    fn run(&mut self) -> Result<Progress<'_>, Failure> {
        self.search.run(limit::PATH.work, limit::PATH.bound);
        let summary = self.search.summary();
        let (outcome, evaluation) = match &self.mode {
            Mode::Follow { target } => (target.then_some(summary.outcome), None),
            Mode::Evaluate { source } => (
                None,
                Some(Evaluation {
                    state: Configuration::from(self.search.current()),
                    source,
                }),
            ),
        };
        Ok(Progress {
            outcome,
            work: summary.work,
            event: summary.length,
            definition: &self.definition,
            evaluation,
        })
    }

    fn state(&self, index: usize) -> Option<Configuration> {
        self.search.inspect(index).map(Configuration::from)
    }

    fn inspect(&self, index: usize) -> Result<Inspection<'_>, Failure> {
        let event = self
            .search
            .transition(index)
            .ok_or_else(|| Failure::new(Code::Request, "No such transition."))?;
        Ok(Inspection {
            event: Step::new(event, &self.definition),
            before: self.state(event.source),
            after: self.state(event.target),
        })
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
            Ok(session) => response::respond(session.run()),
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
