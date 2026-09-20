pub mod dispatch;
pub mod joining;
pub mod partition;
pub mod profile;
pub mod query;
pub mod reachability;
pub mod sharing;
pub mod subscription;

use crate::canonical::Search;
use crate::program::Symbol;
use crate::state::{Frame, State, Token, World};
use serde::Serialize;
use std::hint::black_box;
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
enum Sharing {
    Private,
    Shared,
    Ring,
}

#[derive(Serialize)]
pub struct Measurement {
    sharing: Sharing,
    coherence: usize,
    sample: usize,
    step: usize,
    complete: bool,
    median: f64,
}

fn state(count: usize, sharing: Sharing) -> State {
    let token = |id| Token {
        id,
        value: Symbol::Atom(0),
        capture: None,
    };
    State {
        world: (0..count)
            .map(|index| {
                World {
                    frame: 0,
                    particle: match sharing {
                        Sharing::Private => vec![token(index)],
                        Sharing::Shared => vec![token(0)],
                        Sharing::Ring => vec![token(index), token((index + 1) % count)],
                    },
                }
                .into()
            })
            .collect(),
        frame: vec![
            Frame {
                scope: 0,
                parent: None,
                lexical: None,
                held: Vec::new(),
            }
            .into(),
        ]
        .into(),
    }
}

fn evaluate(state: Arc<State>) -> (usize, bool) {
    let mut search = Search::new(black_box(state));
    for step in 1..=1000 {
        if search.step() {
            black_box(search.finish().unwrap());
            return (step, true);
        }
    }
    (1000, false)
}

pub fn run() -> Vec<Measurement> {
    let mut report = Vec::new();
    for sharing in [Sharing::Private, Sharing::Shared, Sharing::Ring] {
        for count in [2, 4, 8, 30] {
            let state = Arc::new(state(count, sharing));
            let (step, complete) = evaluate(state.clone());
            let mut duration = Vec::new();
            for _ in 0..25 {
                let state = state.clone();
                let start = Instant::now();
                assert_eq!(evaluate(state), (step, complete));
                duration.push(start.elapsed().as_secs_f64() * 1_000_000.0);
            }
            duration.sort_by(f64::total_cmp);
            report.push(Measurement {
                sharing,
                coherence: count,
                sample: duration.len(),
                step,
                complete,
                median: duration[duration.len() / 2],
            });
        }
    }
    report
}
