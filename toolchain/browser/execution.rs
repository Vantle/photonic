use crate::catalog::Catalog;
use crate::configuration::Configuration;
use photonic::place::Place;
use photonic::snapshot::Snapshot;
use photonic::stop::Stop;
use serde::Serialize;

#[derive(Serialize)]
struct Transition {
    id: usize,
    source: usize,
    target: usize,
    rule: String,
    footprint: Vec<Place>,
    exact: Vec<Place>,
    world: Vec<usize>,
    context: Vec<Vec<usize>>,
    deduction: Vec<usize>,
}

// An exploration as the page shows it; stop says why it stopped short of closing, and is empty once
// it closes.
#[derive(Serialize)]
pub struct Execution {
    definition: Catalog,
    closed: bool,
    stop: Vec<Stop>,
    work: usize,
    state: Vec<Configuration>,
    event: Vec<Transition>,
}

impl From<Snapshot> for Execution {
    fn from(snapshot: Snapshot) -> Self {
        let event = snapshot
            .event
            .iter()
            .map(|event| Transition {
                id: event.id,
                source: event.source,
                target: event.target,
                rule: snapshot.definition[event.rule].name.clone(),
                footprint: event.footprint.clone(),
                exact: event.exact.clone(),
                world: event.world.clone(),
                context: event.context.clone(),
                deduction: snapshot.deduction(event.id),
            })
            .collect();
        Self {
            closed: snapshot.closed,
            stop: snapshot.stop.clone(),
            work: snapshot.work,
            state: snapshot
                .state
                .into_iter()
                .map(Configuration::from)
                .collect(),
            definition: Catalog::from(snapshot.definition),
            event,
        }
    }
}
