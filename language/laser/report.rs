use super::Laser;
use super::deduction::Deduction;
use crate::basis::Set;
use crate::place::Place;
use crate::render::Builder;
use crate::runtime::Limit;
use crate::snapshot::{Definition, Form, Link, Node};
use crate::state::{Canonical, State};
use crate::status::Status;
use crate::stop::Stop;
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
    pub stop: Vec<Stop>,
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

// Where an event's binding lies, named as the report names its source: the coherences it binds and
// the places of its footprint, of its exact match and of what it reads.
#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct Placement {
    pub world: Vec<usize>,
    pub footprint: Vec<Place>,
    pub exact: Vec<Place>,
    pub read: Vec<Place>,
}

fn rename(named: &Canonical, set: &Set<Place>) -> Vec<Place> {
    let mut list = set
        .iter()
        .map(|&place| {
            named
                .renaming
                .place(place)
                .expect("a bound place survives renaming")
        })
        .collect::<Vec<_>>();
    list.sort_unstable();
    list
}

impl Laser {
    // Every configuration as a report shows it, in its canonical form when that takes at most the
    // exploration's budget, else listed, with the form each takes.
    pub(super) fn name(&self) -> (Vec<Canonical>, Vec<Form>) {
        self.state
            .iter()
            .map(|state| state.show(self.budget))
            .unzip()
    }

    pub(super) fn placement(&self, index: usize, named: &[Canonical]) -> Placement {
        let binding = &self.identity(index).binding;
        let source = &named[self.event[index].source];
        let mut world = binding
            .world
            .iter()
            .map(|&world| source.renaming.world[world].expect("a bound world survives renaming"))
            .collect::<Vec<_>>();
        world.sort_unstable();
        Placement {
            world,
            footprint: rename(source, &binding.footprint),
            exact: rename(source, &binding.exact),
            read: rename(source, &binding.read),
        }
    }

    fn transition(
        &self,
        index: usize,
        named: &[Canonical],
        status: Status,
        deduction: &Deduction,
    ) -> Transition {
        let event = &self.event[index];
        let Placement {
            world,
            footprint,
            exact,
            read,
        } = self.placement(index, named);
        Transition {
            id: index,
            source: event.source,
            target: event.target,
            rule: self.identity(index).rule,
            status,
            footprint,
            exact,
            read,
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
                    .filter_map(|&origin| source.renaming.place(origin))
                    .collect::<Vec<_>>();
                origin.sort_unstable();
                Some(Link {
                    target: target.renaming.place(value)?,
                    source: origin,
                })
            })
            .collect::<Vec<_>>();
        link.sort_unstable();
        link
    }

    pub fn resource(&self, index: usize) -> Vec<Link> {
        let event = &self.event[index];
        let (source, _) = self.state[event.source].show(self.budget);
        let (target, _) = self.state[event.target].show(self.budget);
        self.link(index, &source, &target)
    }

    pub fn report(&self) -> Report {
        let (named, form) = self.name();
        let (state, event) = self.status();
        let derived = (!self.closed()).then(|| Deduction::derive(self));
        let deduction = derived.as_ref().unwrap_or(&self.deduction);
        let mut builder = Builder::new(&self.program);
        Report {
            definition: builder.definition(),
            closed: self.closed(),
            record: self.retained(),
            peak: self.peak,
            work: self.work,
            limit: self.limit,
            stop: self.stop(),
            state: named
                .iter()
                .zip(form)
                .zip(state)
                .enumerate()
                .map(|(index, ((named, form), status))| {
                    builder.node(index, &named.state, status, form)
                })
                .collect(),
            event: event
                .into_iter()
                .enumerate()
                .map(|(index, status)| self.transition(index, &named, status, deduction))
                .collect(),
        }
    }
}
