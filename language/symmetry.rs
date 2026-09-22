use crate::program::Symbol;
use crate::state::State;
use std::collections::BTreeMap;

#[derive(Eq, Ord, PartialEq, PartialOrd)]
enum Resource {
    Shared(usize, Symbol, Option<usize>, usize),
    Private(Symbol, Option<usize>, usize),
}

pub fn world(state: &State) -> Vec<usize> {
    if state.world.len() < 2 {
        return (0..state.world.len()).collect();
    }
    let mut occurrence = BTreeMap::<usize, usize>::new();
    for token in state
        .world
        .iter()
        .flat_map(|world| &world.particle)
        .chain(state.frame.iter().flat_map(|frame| frame.token()))
    {
        *occurrence.entry(token.id).or_default() += 1;
    }
    let mut representative = BTreeMap::new();
    state
        .world
        .iter()
        .enumerate()
        .map(|(index, world)| {
            let mut local = BTreeMap::new();
            for token in &world.particle {
                let entry = local.entry(token.id).or_insert((token, 0));
                entry.1 += 1;
            }
            let mut resource = local
                .into_iter()
                .map(|(id, (token, count))| {
                    if occurrence[&id] == count {
                        Resource::Private(token.value, token.capture, count)
                    } else {
                        Resource::Shared(id, token.value, token.capture, count)
                    }
                })
                .collect::<Vec<_>>();
            resource.sort();
            *representative
                .entry((world.frame, resource))
                .or_insert(index)
        })
        .collect()
}
