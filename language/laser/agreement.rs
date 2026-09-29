use super::net::{Exploration, Net};
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

impl Laser {
    // A reduced exploration fires, at each configuration, only the events of one scope whose events
    // commute with every other event, so it keeps a subset of the plain configurations and events,
    // every configuration where a plain run ends, and a cycle exactly when the plain one has one.
    pub fn preserves(&self, plain: &Self) -> Result<(), Disagreement> {
        if !self.closed() || !plain.closed() {
            return Err(Disagreement::Closed {
                reference: plain.closed(),
                laser: self.closed(),
            });
        }
        let (outer, inner) = (plain.name(), self.name());
        let known = outer
            .iter()
            .map(|named| &named.state)
            .collect::<HashSet<&State>>();
        let (expected, ending) = (plain.ending(), self.ending());
        let wanted = expected
            .end
            .iter()
            .map(|&index| &outer[index].state)
            .collect::<HashSet<&State>>();
        let found = ending
            .end
            .iter()
            .map(|&index| &inner[index].state)
            .collect::<HashSet<&State>>();
        let stray = inner
            .iter()
            .filter(|named| !known.contains(&named.state))
            .count();
        if wanted != found || stray != 0 {
            return Err(Disagreement::Configuration {
                missing: wanted.difference(&found).count(),
                extra: found.difference(&wanted).count() + stray,
            });
        }
        if expected.endless != ending.endless {
            return Err(Disagreement::Endless {
                reference: expected.endless,
                laser: ending.endless,
            });
        }
        let mut available = BTreeMap::<Edge<'_>, usize>::new();
        for index in 0..plain.event.len() {
            *available
                .entry(Edge::new(plain, &outer, index))
                .or_default() += 1;
        }
        let mut used = BTreeMap::<Edge<'_>, usize>::new();
        for index in 0..self.event.len() {
            *used.entry(Edge::new(self, &inner, index)).or_default() += 1;
        }
        let extra = used
            .iter()
            .map(|(edge, &count)| count.saturating_sub(available.get(edge).copied().unwrap_or(0)))
            .sum::<usize>();
        if extra != 0 {
            return Err(Disagreement::Event { missing: 0, extra });
        }
        Ok(())
    }
}

impl Exploration {
    // Two explorations of one program's net agree number for number when they close alike, find
    // as many configurations and events, take the same work, find the same end configurations in
    // the same order and, where both looked for one, a cycle in both or neither; each net numbers
    // kinds in the order it grounded its parts, so ends are compared by their canonical states.
    pub fn agrees(&self, net: &Net, reference: &Self, theirs: &Net) -> Result<(), Disagreement> {
        if self.closed != reference.closed {
            return Err(Disagreement::Closed {
                reference: reference.closed,
                laser: self.closed,
            });
        }
        if let (Some(endless), Some(expected)) = (self.endless, reference.endless)
            && endless != expected
        {
            return Err(Disagreement::Endless {
                reference: expected,
                laser: endless,
            });
        }
        if self.configuration != reference.configuration {
            return Err(Disagreement::Configuration {
                missing: reference.configuration.saturating_sub(self.configuration),
                extra: self.configuration.saturating_sub(reference.configuration),
            });
        }
        if self.event != reference.event {
            return Err(Disagreement::Event {
                missing: reference.event.saturating_sub(self.event) as usize,
                extra: self.event.saturating_sub(reference.event) as usize,
            });
        }
        if self.work != reference.work {
            return Err(Disagreement::Work {
                reference: reference.work,
                laser: self.work,
            });
        }
        let wanted = reference
            .end
            .iter()
            .map(|marking| theirs.state(marking))
            .collect::<Vec<_>>();
        let found = self
            .end
            .iter()
            .map(|marking| net.state(marking))
            .collect::<Vec<_>>();
        if wanted != found {
            return Err(Disagreement::Configuration {
                missing: wanted.iter().filter(|state| !found.contains(state)).count(),
                extra: found.iter().filter(|state| !wanted.contains(state)).count(),
            });
        }
        Ok(())
    }

    // A net's exploration mirrors the plain engine's when both close with the same number of
    // configurations and events, the same end configurations and a cycle in both or neither.
    pub fn mirrors(&self, net: &Net, plain: &Laser) -> Result<(), Disagreement> {
        let summary = plain.summary();
        if self.closed != summary.closed || !self.closed {
            return Err(Disagreement::Closed {
                reference: summary.closed,
                laser: self.closed,
            });
        }
        let ending = plain.ending();
        if let Some(endless) = self.endless
            && endless != ending.endless
        {
            return Err(Disagreement::Endless {
                reference: ending.endless,
                laser: endless,
            });
        }
        if self.configuration != summary.state {
            return Err(Disagreement::Configuration {
                missing: summary.state.saturating_sub(self.configuration),
                extra: self.configuration.saturating_sub(summary.state),
            });
        }
        let expected = summary.event as u64;
        if self.event != expected {
            return Err(Disagreement::Event {
                missing: expected.saturating_sub(self.event) as usize,
                extra: self.event.saturating_sub(expected) as usize,
            });
        }
        let mut wanted = ending
            .end
            .iter()
            .map(|&index| plain.state[index].canonical().state)
            .collect::<Vec<_>>();
        let mut found = self
            .end
            .iter()
            .map(|marking| net.state(marking))
            .collect::<Vec<_>>();
        wanted.sort();
        found.sort();
        if wanted != found {
            return Err(Disagreement::Configuration {
                missing: wanted.iter().filter(|state| !found.contains(state)).count(),
                extra: found.iter().filter(|state| !wanted.contains(state)).count(),
            });
        }
        Ok(())
    }
}
