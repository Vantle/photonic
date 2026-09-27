use super::Laser;
use crate::basis::Set;
use crate::place::Place;
use crate::render::Builder;
use crate::runtime::Limit;
use crate::snapshot::{Definition, Node};
use crate::state::Canonical;
use crate::status::Status;
use serde::Serialize;

// Laser names configurations by their components, so a report renames each configuration to the
// interpreter's canonical form and every event's places with it: the same configuration reads the
// same whichever engine found it.
#[derive(Debug, Serialize)]
pub struct Report {
    pub definition: Vec<Definition>,
    pub closed: bool,
    pub record: usize,
    pub peak: usize,
    pub work: usize,
    pub limit: Limit,
    pub state: Vec<Node>,
    pub event: Vec<Transition>,
}

#[derive(Debug, Serialize)]
pub struct Transition {
    pub id: usize,
    pub source: usize,
    pub target: usize,
    pub rule: usize,
    pub status: Status,
    pub footprint: Vec<Place>,
    pub exact: Vec<Place>,
    pub read: Vec<Place>,
    pub inferred: bool,
    pub world: Vec<usize>,
}

fn rename(named: &Canonical, set: &Set<Place>) -> Vec<Place> {
    let mut list = set
        .iter()
        .map(|&place| named.place(place).expect("a bound place survives renaming"))
        .collect::<Vec<_>>();
    list.sort_unstable();
    list
}

impl Laser {
    pub(super) fn name(&self) -> Vec<Canonical> {
        self.state.iter().map(|state| state.canonical()).collect()
    }

    pub(super) fn transition(
        &self,
        index: usize,
        named: &[Canonical],
        status: Status,
    ) -> Transition {
        let event = &self.event[index];
        let identity = self.identity(index);
        let binding = &identity.binding;
        let source = &named[event.source];
        let mut world = binding
            .world
            .iter()
            .map(|&index| source.world[index].expect("a bound world survives renaming"))
            .collect::<Vec<_>>();
        world.sort_unstable();
        Transition {
            id: index,
            source: event.source,
            target: event.target,
            rule: identity.rule,
            status,
            footprint: rename(source, &binding.footprint),
            exact: rename(source, &binding.exact),
            read: rename(source, &binding.read),
            inferred: !event.direct,
            world,
        }
    }

    pub fn report(&self) -> Report {
        let named = self.name();
        let (state, event) = self.status();
        let mut builder = Builder::new(&self.program);
        Report {
            definition: builder.definition(),
            closed: self.closed(),
            record: self.record(),
            peak: self.peak,
            work: self.work,
            limit: self.limit,
            state: named
                .iter()
                .zip(state)
                .enumerate()
                .map(|(index, (named, status))| builder.node(index, &named.state, status))
                .collect(),
            event: event
                .into_iter()
                .enumerate()
                .map(|(index, status)| self.transition(index, &named, status))
                .collect(),
        }
    }
}
