use crate::population::Set;
use crate::state::State;

fn extent(value: &Set, previous: Option<&Set>) -> usize {
    if let Some(previous) = previous.filter(|previous| value.shared(previous)) {
        return value
            .removed(previous)
            .into_iter()
            .map(|position| value.at(position).id + 1)
            .max()
            .unwrap_or(0);
    }
    value.iter().map(|token| token.id + 1).max().unwrap_or(0)
}

pub(super) fn bound(current: &State, previous: &State, frame: &[usize]) -> usize {
    frame
        .iter()
        .filter_map(|&position| current.frame.get(position).map(|frame| (position, frame)))
        .map(|(position, frame)| {
            let previous = previous.frame.get(position).map(|frame| &frame.particle);
            frame
                .held
                .iter()
                .map(|token| token.id + 1)
                .max()
                .unwrap_or(0)
                .max(extent(&frame.particle, previous))
        })
        .max()
        .unwrap_or(0)
}
