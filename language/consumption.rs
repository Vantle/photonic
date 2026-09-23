use crate::flow::Binding;
use crate::place::Place;
use crate::state::State;
use smallvec::SmallVec;
use std::sync::Arc;

pub(crate) fn apply(state: &mut State, binding: &Binding) -> Vec<usize> {
    let selection = binding
        .exact
        .iter()
        .filter_map(|&place| match place {
            Place::Context(frame, resource) => Some((frame, resource)),
            _ => None,
        })
        .collect::<SmallVec<[(usize, usize); 4]>>();
    selection
        .chunk_by(|left, right| left.0 == right.0)
        .map(|group| {
            let frame = group[0].0;
            Arc::make_mut(&mut state.frame[frame])
                .particle
                .retain(|token| group.iter().all(|&(_, resource)| resource != token.id));
            frame
        })
        .collect()
}
