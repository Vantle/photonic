use super::{Disagreement, Laser};
use crate::place::Place;
use crate::program::Symbol;
use crate::runtime::Runtime;
use crate::snapshot::Link;
use crate::state::{State, Token};
use crate::status::Status;
use std::collections::{BTreeMap, HashMap, HashSet};

// A configuration's automorphisms exchange equal tokens and equal containers, and each engine's
// canonical labeling may settle on a different one, so place maps are compared by what every
// automorphism keeps: each place's kind, its token and the tokens beside it.
#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Class {
    kind: u8,
    value: (Symbol, bool),
    container: Vec<(Symbol, bool)>,
}

fn value(token: &Token) -> (Symbol, bool) {
    (token.value, token.capture.is_some())
}

fn class(state: &State, place: Place) -> Class {
    let (kind, container) = match place {
        Place::World(index, _) => (0, state.world[index].particle.iter().collect::<Vec<_>>()),
        Place::Context(index, _) | Place::Held(index, _) => {
            let frame = &state.frame[index];
            let kind = if matches!(place, Place::Context(..)) {
                1
            } else {
                2
            };
            (kind, frame.particle.iter().chain(&frame.held).collect())
        }
    };
    let mut container = container.into_iter().map(value).collect::<Vec<_>>();
    container.sort_unstable();
    Class {
        kind,
        value: value(state.token(place).expect("a linked place holds a token")),
        container,
    }
}

fn shape(source: &State, target: &State, link: &[Link]) -> Vec<(Class, Vec<Class>)> {
    let mut shape = link
        .iter()
        .map(|link| {
            let mut origin = link
                .source
                .iter()
                .map(|&place| class(source, place))
                .collect::<Vec<_>>();
            origin.sort_unstable();
            (class(target, link.target), origin)
        })
        .collect::<Vec<_>>();
    shape.sort_unstable();
    shape
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Key {
    source: usize,
    target: usize,
    rule: usize,
    world: Vec<usize>,
    footprint: Vec<Place>,
    exact: Vec<Place>,
    read: Vec<Place>,
    direct: bool,
    supported: bool,
    resource: Vec<(Class, Vec<Class>)>,
}

fn difference<Value: Ord>(
    reference: &BTreeMap<Value, usize>,
    observed: &BTreeMap<Value, usize>,
) -> (usize, usize) {
    let count = |from: &BTreeMap<Value, usize>, to: &BTreeMap<Value, usize>| {
        from.iter()
            .map(|(key, &count)| count.saturating_sub(to.get(key).copied().unwrap_or(0)))
            .sum::<usize>()
    };
    (count(reference, observed), count(observed, reference))
}

impl Laser {
    pub fn agree(&self, runtime: &Runtime) -> Result<(), Disagreement> {
        if runtime.closed() != self.closed() {
            return Err(Disagreement::Closed {
                reference: runtime.closed(),
                laser: self.closed(),
            });
        }
        let reference = runtime
            .state
            .iter()
            .map(AsRef::as_ref)
            .collect::<HashSet<&State>>();
        let named = self.name();
        let observed = named
            .iter()
            .map(|named| &named.state)
            .collect::<HashSet<&State>>();
        if reference != observed {
            return Err(Disagreement::Configuration {
                missing: reference.difference(&observed).count(),
                extra: observed.difference(&reference).count(),
            });
        }
        let number = runtime
            .state
            .iter()
            .enumerate()
            .map(|(index, state)| (state.as_ref(), index))
            .collect::<HashMap<&State, usize>>();
        let snapshot = runtime.snapshot();
        let (state, status) = self.status();
        let differ = named
            .iter()
            .zip(&state)
            .filter(|(named, status)| snapshot.state[number[&named.state]].status != **status)
            .count();
        if differ != 0 {
            return Err(Disagreement::Support {
                configuration: differ,
            });
        }
        let mut expected = BTreeMap::<Key, usize>::new();
        for event in &snapshot.event {
            let direct = event
                .evidence
                .iter()
                .any(|&view| snapshot.view[view].source == snapshot.view[view].target);
            let key = Key {
                source: event.source,
                target: event.target,
                rule: event.rule,
                world: event.world.clone(),
                footprint: event.footprint.clone(),
                exact: event.exact.clone(),
                read: event.read.clone(),
                direct,
                supported: event.status == Status::Supported,
                resource: shape(
                    &runtime.state[event.source],
                    &runtime.state[event.target],
                    &runtime.resource(event.id).unwrap_or_default(),
                ),
            };
            *expected.entry(key).or_default() += 1;
        }
        let derived = self.derived();
        let deduction = derived.as_ref().unwrap_or(&self.deduction);
        let mut compiled = BTreeMap::<Key, usize>::new();
        for (index, &status) in status.iter().enumerate() {
            let transition = self.transition(index, &named, status, deduction);
            let (source, target) = (&named[transition.source], &named[transition.target]);
            let resource = shape(
                &source.state,
                &target.state,
                &self.link(index, source, target),
            );
            let key = Key {
                source: number[&named[transition.source].state],
                target: number[&named[transition.target].state],
                rule: transition.rule,
                world: transition.world,
                footprint: transition.footprint,
                exact: transition.exact,
                read: transition.read,
                direct: !transition.inferred,
                supported: transition.status == Status::Supported,
                resource,
            };
            *compiled.entry(key).or_default() += 1;
        }
        let (missing, extra) = difference(&expected, &compiled);
        if missing != 0 || extra != 0 {
            return Err(Disagreement::Event { missing, extra });
        }
        Ok(())
    }
}

// A plain exploration is the part of a full one that matched events reach from the start: the same
// configurations, and among them the same events.
fn part(full: &Laser) -> Vec<bool> {
    let mut outgoing = vec![Vec::new(); full.state.len()];
    for (index, event) in full.event.iter().enumerate() {
        if event.matched {
            outgoing[event.source].push(index);
        }
    }
    let mut reached = vec![false; full.state.len()];
    reached[0] = true;
    let mut pending = vec![0];
    while let Some(state) = pending.pop() {
        for &event in &outgoing[state] {
            let target = full.event[event].target;
            if !std::mem::replace(&mut reached[target], true) {
                pending.push(target);
            }
        }
    }
    reached
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Edge<'state> {
    source: &'state State,
    target: &'state State,
    rule: usize,
    world: Vec<usize>,
    footprint: Vec<Place>,
    exact: Vec<Place>,
    read: Vec<Place>,
}

impl<'state> Edge<'state> {
    fn new(laser: &Laser, named: &'state [crate::state::Canonical], index: usize) -> Self {
        let transition = laser.transition(index, named, Status::Supported, &laser.deduction);
        Self {
            source: &named[transition.source].state,
            target: &named[transition.target].state,
            rule: transition.rule,
            world: transition.world,
            footprint: transition.footprint,
            exact: transition.exact,
            read: transition.read,
        }
    }
}

impl Laser {
    pub fn within(&self, full: &Self) -> Result<(), Disagreement> {
        if !self.closed() || !full.closed() {
            return Err(Disagreement::Closed {
                reference: full.closed(),
                laser: self.closed(),
            });
        }
        let reached = part(full);
        let (outer, inner) = (full.name(), self.name());
        let expected = (0..full.state.len())
            .filter(|&index| reached[index])
            .map(|index| &outer[index].state)
            .collect::<HashSet<&State>>();
        let observed = inner
            .iter()
            .map(|named| &named.state)
            .collect::<HashSet<&State>>();
        if expected != observed {
            return Err(Disagreement::Configuration {
                missing: expected.difference(&observed).count(),
                extra: observed.difference(&expected).count(),
            });
        }
        let mut wanted = BTreeMap::<Edge<'_>, usize>::new();
        for (index, event) in full.event.iter().enumerate() {
            if event.matched && reached[event.source] {
                *wanted.entry(Edge::new(full, &outer, index)).or_default() += 1;
            }
        }
        let mut found = BTreeMap::<Edge<'_>, usize>::new();
        for index in 0..self.event.len() {
            *found.entry(Edge::new(self, &inner, index)).or_default() += 1;
        }
        let (missing, extra) = difference(&wanted, &found);
        if missing != 0 || extra != 0 {
            return Err(Disagreement::Event { missing, extra });
        }
        Ok(())
    }
}
