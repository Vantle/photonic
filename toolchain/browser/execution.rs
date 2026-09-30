use crate::catalog::Catalog;
use crate::configuration::Configuration;
use photonic::place::Place;
use photonic::snapshot::Snapshot;
use photonic::stop::Stop;
use serde::Serialize;
use spectrum::numbering::Numbering;
use spectrum::order::Naming;

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

impl Execution {
    // Every configuration and event under the handle Spectrum's answers give it, listed in that
    // order, with each event's source, target and deduction renumbered alike.
    pub fn new(
        snapshot: Snapshot,
        numbering: &Numbering,
        naming: &Naming,
        definition: Catalog,
    ) -> Self {
        let count = snapshot.event.len();
        let mut position = vec![0; count];
        for canonical in 0..count {
            position[numbering.event(canonical)] = canonical;
        }
        let event = (0..count)
            .map(|canonical| {
                let value = &snapshot.event[numbering.event(canonical)];
                Transition {
                    id: canonical,
                    source: numbering.configuration(value.source),
                    target: numbering.configuration(value.target),
                    rule: definition.name(value.rule).to_owned(),
                    footprint: value.footprint.clone(),
                    exact: value.exact.clone(),
                    world: value.world.clone(),
                    context: value.context.clone(),
                    deduction: snapshot
                        .deduction(value.id)
                        .into_iter()
                        .map(|step| position[step])
                        .collect(),
                }
            })
            .collect();
        let mut state = snapshot
            .state
            .into_iter()
            .map(|node| {
                let id = numbering.configuration(node.id);
                Configuration::new(node, id, naming)
            })
            .collect::<Vec<_>>();
        state.sort_by_key(Configuration::id);
        Self {
            definition,
            closed: snapshot.closed,
            stop: snapshot.stop,
            work: snapshot.work,
            state,
            event,
        }
    }
}
