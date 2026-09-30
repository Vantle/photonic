use crate::budget::Budget;
use crate::context::Context;
use crate::exploration::{Exploration, Plan};
use crate::explore;
use crate::explored::Explored;
use crate::failure::{Code, Failure};
use crate::recording::{Engine, Mode, Recording};
use crate::store::Store;
use crate::subject::{Reader, Subject};
use std::sync::Arc;

struct Source(&'static str);

impl Reader for Source {
    fn read(&self, path: &str) -> Result<String, Failure> {
        (path == "program.wave")
            .then(|| self.0.to_owned())
            .ok_or_else(|| Failure::new(Code::File, format!("{path} is not in memory")))
    }
}

fn summary(source: &str, mode: Mode, engine: Engine, budget: Budget) -> String {
    let program = frontend::lowering::parse(source).unwrap();
    let explored = Explored::Exploration(Arc::new(Exploration::new(Plan::new(
        &program, mode, engine, budget, None,
    ))));
    explore::state(&explore::brief(&explored))
}

// An open exploration says which budget or limit stopped it and what to raise, a start that already
// passes a limit says what it holds, and a direct path says why it stopped.
#[test]
fn wording() {
    let small = Budget {
        occurrence: 3,
        ..Budget::default()
    };
    for (source, mode, engine, budget, said) in [
        (
            "A, [A] A.A",
            Mode::Exhaustive,
            Engine::Laser,
            small,
            "open: the occurrence limit (3) blocked 3 events; raise --occurrence",
        ),
        (
            "A.A.A.A.A, [A.A] A",
            Mode::Exhaustive,
            Engine::Interpreter,
            small,
            "open: s0 holds 5 occurrences; the occurrence limit is 3; raise --occurrence",
        ),
        (
            "A, [A] A.A",
            Mode::Plain,
            Engine::Laser,
            Budget {
                work: 4,
                ..Budget::default()
            },
            "open: the work budget (4) ran out; raise --work",
        ),
        (
            "A, [A] B",
            Mode::Path,
            Engine::Interpreter,
            Budget::default(),
            "path stopped: no event applies",
        ),
        (
            "A, [A] A.A",
            Mode::Path,
            Engine::Interpreter,
            small,
            "path stopped: the occurrence limit (3) blocked 3 events; raise --occurrence",
        ),
        ("A, [A] B", Mode::Exhaustive, Engine::Laser, small, "closed"),
    ] {
        let text = summary(source, mode, engine, budget);
        assert!(text.contains(said), "{source}: {text}");
    }
}

// Metal keeps no records, so the record budget leaves its key alone.
#[test]
fn record() {
    let reader = Source("Light, [Light] Red, [Light] Green");
    let mut store = Store::default();
    let mut context = Context {
        reader: &reader,
        store: &mut store,
    };
    let key = |context: &mut Context<'_>, record: usize| {
        let recording = Recording {
            program: Some(Subject {
                file: vec!["program.wave".to_owned()],
                ..Subject::default()
            }),
            mode: Some(Mode::Plain),
            engine: Some(Engine::Metal),
            budget: Some(Budget {
                record,
                ..Budget::default()
            }),
            ..Recording::default()
        };
        context.explored(&recording).unwrap().key().to_owned()
    };
    assert_eq!(key(&mut context, 7), key(&mut context, 2_000_000));
}
