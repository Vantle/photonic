use crate::runtime::Measure;
use crate::state::State;

// What a component of a kind holds beside the root: its coherences, frames, token ids and the
// occurrences the occurrence limit weighs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Size {
    pub world: usize,
    pub frame: usize,
    pub token: usize,
    pub occurrence: usize,
}

impl Size {
    // A component extracted with a stand-in root, whose frame is not the component's own.
    pub(super) fn new(state: &State) -> Self {
        let mut token = state
            .world
            .iter()
            .flat_map(|world| world.particle.iter())
            .chain(state.frame.iter().flat_map(|frame| frame.token()))
            .map(|token| token.id)
            .collect::<Vec<_>>();
        token.sort_unstable();
        token.dedup();
        Self {
            world: state.world.len(),
            frame: state.frame.len() - 1,
            token: token.len(),
            occurrence: Measure::new(state).occurrence,
        }
    }
}
