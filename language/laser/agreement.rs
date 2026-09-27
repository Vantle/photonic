use super::{Disagreement, Laser};
use crate::place::Place;
use crate::runtime::Runtime;
use crate::state::State;
use std::collections::{BTreeMap, HashMap, HashSet};

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
}

fn difference(reference: &BTreeMap<Key, usize>, observed: &BTreeMap<Key, usize>) -> (usize, usize) {
    let count = |from: &BTreeMap<Key, usize>, to: &BTreeMap<Key, usize>| {
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
                interpreter: runtime.closed(),
                laser: self.closed(),
            });
        }
        let reference = runtime
            .state
            .iter()
            .map(AsRef::as_ref)
            .collect::<HashSet<&State>>();
        let observed = self
            .state
            .iter()
            .map(AsRef::as_ref)
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
            };
            *expected.entry(key).or_default() += 1;
        }
        let mut compiled = BTreeMap::<Key, usize>::new();
        for (index, event) in self.event.iter().enumerate() {
            let identity = self.identity(index);
            let binding = &identity.binding;
            let key = Key {
                source: number[self.state[event.source].as_ref()],
                target: number[self.state[event.target].as_ref()],
                rule: identity.rule,
                world: binding.world.iter().copied().collect(),
                footprint: binding.footprint.iter().copied().collect(),
                exact: binding.exact.iter().copied().collect(),
                read: binding.read.iter().copied().collect(),
                direct: event.direct,
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
