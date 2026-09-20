use crate::program::{Instruction, Symbol};

pub(crate) struct Emission {
    pub value: Symbol,
    pub capture: bool,
}

pub(crate) struct Output {
    pub emission: Vec<Emission>,
    pub scope: Option<usize>,
}

pub(crate) struct Recipe {
    pub output: Vec<Output>,
    pub nested: bool,
}

impl Recipe {
    pub fn new(instruction: &Instruction) -> Self {
        Self {
            output: instruction
                .output
                .iter()
                .map(|output| Output {
                    emission: output
                        .particle
                        .iter()
                        .map(|&value| Emission {
                            value,
                            capture: matches!(value, Symbol::Rule(_)),
                        })
                        .collect(),
                    scope: output.body,
                })
                .collect(),
            nested: instruction
                .output
                .iter()
                .any(|output| output.body.is_some()),
        }
    }

    pub fn retained(&self) -> usize {
        self.output
            .iter()
            .map(|output| output.emission.len() + 1)
            .sum()
    }
}
