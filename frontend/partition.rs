use crate::source::{Definition, Output, Value};

pub(crate) struct Partition {
    pub pattern: Vec<Vec<Vec<Value>>>,
    pub output: Vec<Output>,
}

impl Partition {
    pub(crate) fn rule(self, budget: &mut usize) -> Option<Vec<Definition>> {
        if self.pattern.len() == 1 {
            let input = self.pattern.into_iter().next()?;
            return Some(vec![Definition {
                input,
                output: self.output,
            }]);
        }
        let mut rule = Vec::new();
        for (index, input) in self.pattern.iter().enumerate() {
            for output in self.target(index) {
                let definition = Definition {
                    input: input.clone(),
                    output,
                };
                *budget = budget.checked_sub(crate::size::definition(&definition))?;
                rule.push(definition);
            }
        }
        Some(rule)
    }

    fn target(&self, source: usize) -> impl Iterator<Item = Vec<Output>> {
        self.pattern
            .iter()
            .enumerate()
            .filter(move |&(index, _)| index != source)
            .map(|(_, pattern)| {
                pattern
                    .iter()
                    .map(|particle| Output::Particle(particle.clone()))
                    .collect()
            })
            .chain((!self.output.is_empty()).then(|| self.output.clone()))
    }
}
