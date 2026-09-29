use crate::state::State;

// A scope encloses itself and every frame whose enclosing or defining frame it encloses, so nothing
// outside can bind their occurrences, add to them or apply their rules without a capture.
pub(super) struct Enclosure {
    scope: usize,
    frame: Vec<bool>,
}

impl Enclosure {
    pub fn new(state: &State, scope: usize) -> Self {
        let mut frame = vec![false; state.frame.len()];
        frame[scope] = true;
        let mut grown = true;
        while grown {
            grown = false;
            for (index, value) in state.frame.iter().enumerate() {
                if !frame[index]
                    && value
                        .parent
                        .into_iter()
                        .chain(value.lexical)
                        .any(|link| frame[link])
                {
                    frame[index] = true;
                    grown = true;
                }
            }
        }
        Self { scope, frame }
    }

    pub fn scope(&self) -> usize {
        self.scope
    }

    pub fn holds(&self, frame: usize) -> bool {
        self.frame[frame]
    }

    // A sealed scope has no occurrence outside it that captures one of its frames, so no rule outside
    // applies, through a capture, what the scope's events change.
    pub fn sealed(&self, state: &State) -> bool {
        let world = state
            .world
            .iter()
            .filter(|world| !self.frame[world.frame])
            .flat_map(|world| world.particle.iter());
        let frame = state
            .frame
            .iter()
            .enumerate()
            .filter(|(index, _)| !self.frame[*index])
            .flat_map(|(_, frame)| frame.token());
        !world
            .chain(frame)
            .any(|token| token.capture.is_some_and(|capture| self.frame[capture]))
    }
}
