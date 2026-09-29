use crate::program::{Program, Symbol};
use crate::state::World;

// A rule whose input names a rule that is live in some scope can consume a live rule of any frame,
// even the root, and change what every event may do, so a program with one has no independence.
// Without one, live rules never change, and only a rule with several input coherences, live or held
// as a value, can bind a coherence together with another.
pub(super) struct Independence {
    join: Vec<Vec<Symbol>>,
}

impl Independence {
    pub fn new(program: &Program) -> Option<Self> {
        let contextual = program.contextual();
        let live = program
            .rule
            .iter()
            .flat_map(|rule| rule.input.iter().flatten())
            .any(|symbol| matches!(symbol, Symbol::Rule(index) if contextual[*index]));
        if live {
            return None;
        }
        let join = program
            .rule
            .iter()
            .filter(|rule| rule.input.len() > 1)
            .flat_map(|rule| rule.input.iter().cloned())
            .collect();
        Some(Self { join })
    }

    // A coherence can be joined when an input coherence of such a rule names only what it holds.
    pub fn joinable(&self, world: &World) -> bool {
        self.join.iter().any(|pattern| {
            pattern
                .iter()
                .all(|symbol| world.particle.iter().any(|token| token.value == *symbol))
        })
    }
}
