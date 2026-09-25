use crate::catalog::Catalog;
use crate::configuration::Configuration;
use crate::failure::Failure;
use photonic::place::Place;
use photonic::snapshot::{Event, Snapshot, View};
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

#[derive(Serialize)]
pub struct Execution {
    definition: Catalog,
    closed: bool,
    work: usize,
    state: Vec<Configuration>,
    event: Vec<Transition>,
}

fn deduction(view: &[View], evidence: &[usize]) -> Vec<usize> {
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
}

impl Transition {
    fn new(event: Event, view: &[View]) -> Self {
        Self {
            deduction: deduction(view, &event.evidence),
            id: event.id,
            source: event.source,
            target: event.target,
            rule: event.rule,
            footprint: event.footprint,
            exact: event.exact,
            world: event.world,
            context: event.context,
        }
    }
}

impl TryFrom<Snapshot> for Execution {
    type Error = Failure;

    fn try_from(snapshot: Snapshot) -> Result<Self, Failure> {
        let definition = Catalog::from(snapshot.definition);
        let view = snapshot.view;
        Ok(Self {
            state: snapshot
                .state
                .into_iter()
                .map(|node| Configuration::new(node, &definition))
                .collect::<Result<_, _>>()?,
            definition,
            closed: snapshot.closed,
            work: snapshot.work,
            event: snapshot
                .event
                .into_iter()
                .map(|event| Transition::new(event, &view))
                .collect(),
        })
    }
}
