use super::Laser;
use super::deduction::Deduction;
use crate::basis::Set;
use crate::place::Place;
use crate::render::Builder;
use crate::runtime::Limit;
use crate::snapshot::{Definition, Link, Node};
use crate::state::{Canonical, State};
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
    pub deduction: Vec<usize>,
    pub world: Vec<usize>,
}

fn place(state: &State) -> impl Iterator<Item = Place> + '_ {
    let world = state.world.iter().enumerate().flat_map(|(index, world)| {
        world
            .particle
            .iter()
            .map(move |token| Place::World(index, token.id))
    });
    let frame = state.frame.iter().enumerate().flat_map(|(index, frame)| {
        let context = frame
            .particle
            .iter()
            .map(move |token| Place::Context(index, token.id));
        let held = frame
            .held
            .iter()
            .map(move |token| Place::Held(index, token.id));
        context.chain(held)
    });
    world.chain(frame)
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

    pub(super) fn derived(&self) -> Option<Deduction> {
        (!self.closed()).then(|| Deduction::derive(self))
    }

    pub(super) fn transition(
        &self,
        index: usize,
        named: &[Canonical],
        status: Status,
        deduction: &Deduction,
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
            deduction: deduction.get(index),
            world,
        }
    }

    // Each place after an event and the places it came from before, both named as the report names
    // its configurations.
    pub(super) fn link(&self, index: usize, source: &Canonical, target: &Canonical) -> Vec<Link> {
        let event = &self.event[index];
        let passage = &self.passage[index];
        let mut link = place(&self.state[event.target])
            .filter_map(|value| {
                let mut origin = passage
                    .resource(value)
                    .iter()
                    .filter_map(|&origin| source.place(origin))
                    .collect::<Vec<_>>();
                origin.sort_unstable();
                Some(Link {
                    target: target.place(value)?,
                    source: origin,
                })
            })
            .collect::<Vec<_>>();
        link.sort_unstable();
        link
    }

    pub fn resource(&self, index: usize) -> Vec<Link> {
        let event = &self.event[index];
        self.link(
            index,
            &self.state[event.source].canonical(),
            &self.state[event.target].canonical(),
        )
    }

    pub fn report(&self) -> Report {
        let named = self.name();
        let (state, event) = self.status();
        let derived = self.derived();
        let deduction = derived.as_ref().unwrap_or(&self.deduction);
        let mut builder = Builder::new(&self.program);
        Report {
            definition: builder.definition(),
            closed: self.closed(),
            record: self.retained(),
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
                .map(|(index, status)| self.transition(index, &named, status, deduction))
                .collect(),
        }
    }
}
