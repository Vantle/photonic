use crate::index::Index;
use crate::program::{Program, Symbol};
use crate::search::Search;
use crate::selection::Store;
use crate::state::{State, Token, World};
use crate::term::Term;
use serde::Serialize;
use std::hint::black_box;
use std::sync::Arc;
use std::task::Poll;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
enum Mode {
    Atom,
    Rule,
    Context,
}

#[derive(Serialize)]
pub struct Measurement {
    mode: Mode,
    width: usize,
    shared: bool,
    reused: bool,
    work: usize,
    retained: usize,
    sample: Vec<f64>,
}

fn fixture(width: usize, mode: Mode) -> (State, Vec<Vec<Term>>) {
    let source = (0..width)
        .map(|position| format!("[Value{position}] Result{position}"))
        .collect::<Vec<_>>()
        .join(" ");
    let program = Program::new(&crate::lowering::parse(&source).unwrap());
    let mut state = State::initial(&program);
    Arc::make_mut(&mut state.frame[0]).particle.clear();
    if matches!(mode, Mode::Context) {
        Arc::make_mut(&mut state.frame[0]).particle = (0..width)
            .map(|position| Token {
                id: position + width,
                value: Symbol::Rule(0),
                capture: Some(0),
            })
            .collect();
        let mut frame = (*state.frame[0]).clone();
        frame.particle.clear();
        frame.parent = Some(0);
        frame.lexical = Some(0);
        state.frame.push(frame.into());
        state.world = vec![
            Arc::new(World {
                frame: 1,
                particle: (0..width)
                    .map(|id| Token {
                        id,
                        value: Symbol::Atom(0),
                        capture: None,
                    })
                    .collect(),
            }),
            Arc::new(World {
                frame: 0,
                particle: vec![Token {
                    id: width * 2,
                    value: Symbol::Atom(1),
                    capture: None,
                }],
            }),
        ]
        .into();
        return (
            state,
            vec![
                vec![Term::new(Symbol::Rule(0), Some(0)); width],
                vec![Term::new(Symbol::Atom(0), None); width],
            ],
        );
    }
    let term = |position| match mode {
        Mode::Atom => Term::new(Symbol::Atom(position), None),
        Mode::Rule => Term::new(Symbol::Rule(position), Some(0)),
        Mode::Context => unreachable!(),
    };
    state.world = (0..=width)
        .map(|position| {
            let term = if position == width {
                Term::new(Symbol::Atom(width + 1), None)
            } else {
                term(position)
            };
            Arc::new(World {
                frame: 0,
                particle: vec![Token {
                    id: position,
                    value: term.value,
                    capture: term.capture,
                }],
            })
        })
        .collect();
    (
        state,
        (0..width).map(|position| vec![term(position)]).collect(),
    )
}

pub fn run() -> Vec<Measurement> {
    let mut report = Vec::new();
    for mode in [Mode::Atom, Mode::Rule, Mode::Context] {
        for width in [8, 32, 128, 512] {
            let (state, pattern) = fixture(width, mode);
            let frame = usize::from(matches!(mode, Mode::Context));
            let arity = pattern.len();
            let mut other = state.clone();
            let last = other.world.len() - 1;
            Arc::make_mut(&mut other.world[last]).particle[0].id += 1;
            let index = [state, other].map(|state| Arc::new(Index::new(Arc::new(state))));
            for (shared, reused) in [(false, false), (true, false), (true, true)] {
                let evaluate = || {
                    let mut store = Arc::new(Store::new(65536));
                    let start = Instant::now();
                    let mut work = 0;
                    for iteration in 0..64 {
                        if !reused {
                            store = Arc::new(Store::new(65536));
                        }
                        let mut search = if shared {
                            Search::shared(
                                pattern.clone(),
                                index[iteration % 2].clone(),
                                frame,
                                &store,
                            )
                        } else {
                            Search::new(pattern.clone(), index[iteration % 2].clone(), frame)
                        };
                        let mut binding = 0;
                        loop {
                            work += 1;
                            match black_box(search.step()) {
                                Poll::Ready(None) => break,
                                Poll::Ready(Some(value)) => {
                                    assert_eq!(value.len(), arity);
                                    binding += 1;
                                }
                                Poll::Pending => {}
                            }
                        }
                        assert_eq!(binding, 1);
                    }
                    (start.elapsed().as_secs_f64(), work, store.retained())
                };
                let (_, work, retained) = evaluate();
                let warm = Instant::now();
                while warm.elapsed() < Duration::from_millis(100) {
                    evaluate();
                }
                report.push(Measurement {
                    mode,
                    width,
                    shared,
                    reused,
                    work,
                    retained,
                    sample: (0..9).map(|_| evaluate().0).collect(),
                });
            }
        }
    }
    report
}
