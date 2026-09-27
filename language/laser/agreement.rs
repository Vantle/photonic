use super::{Disagreement, Laser};
use crate::place::Place;
use crate::runtime::Runtime;
use crate::state::State;
use crate::status::Status;
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
    supported: bool,
}

fn difference(reference: &BTreeMap<Key, usize>, observed: &BTreeMap<Key, usize>) -> (usize, usize) {
    let count = |from: &BTreeMap<Key, usize>, to: &BTreeMap<Key, usize>| {
        from.iter()
            .map(|(key, &count)| count.saturating_sub(to.get(key).copied().unwrap_or(0)))
            .sum::<usize>()
    };
    (count(reference, observed), count(observed, reference))
}

// Laser names configurations by their components, so each is renamed to the interpreter's name
// before comparing, and every event's places move with the renaming of its source.
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
        let named = self
            .state
            .iter()
            .map(|state| state.canonical())
            .collect::<Vec<_>>();
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
            };
            *expected.entry(key).or_default() += 1;
        }
        let mut compiled = BTreeMap::<Key, usize>::new();
        for (index, event) in self.event.iter().enumerate() {
            let identity = self.identity(index);
            let binding = &identity.binding;
            let source = &named[event.source];
            let rename = |set: &crate::basis::Set<Place>| {
                let mut list = set
                    .iter()
                    .map(|&place| {
                        source
                            .place(place)
                            .expect("a bound place survives renaming")
                    })
                    .collect::<Vec<_>>();
                list.sort_unstable();
                list
            };
            let mut world = binding
                .world
                .iter()
                .map(|&index| source.world[index].expect("a bound world survives renaming"))
                .collect::<Vec<_>>();
            world.sort_unstable();
            let key = Key {
                source: number[&source.state],
                target: number[&named[event.target].state],
                rule: identity.rule,
                world,
                footprint: rename(&binding.footprint),
                exact: rename(&binding.exact),
                read: rename(&binding.read),
                direct: event.direct,
                supported: status[index] == Status::Supported,
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
