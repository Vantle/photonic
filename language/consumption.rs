use crate::flow::{Binding, Place};
use crate::state::State;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

pub(crate) fn apply(state: &mut State, binding: &Binding) -> Vec<usize> {
    let mut selection = BTreeMap::<_, BTreeSet<_>>::new();
    for &place in &binding.exact {
        if let Place::Context(frame, resource) = place {
            selection.entry(frame).or_default().insert(resource);
        }
    }
    selection
        .into_iter()
        .map(|(frame, selection)| {
            Arc::make_mut(&mut state.frame[frame])
                .particle
                .retain(|token| !selection.contains(&token.id));
            frame
        })
        .collect()
}
