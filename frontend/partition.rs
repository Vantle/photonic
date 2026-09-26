use std::ops::Range;

use crate::source::{Definition, Output, Value};

pub(crate) struct Partition<'source> {
    pub source: &'source str,
    pub span: Range<usize>,
    pub pattern: Vec<Vec<Vec<Value>>>,
    pub output: Vec<Output>,
}

impl Partition<'_> {
    pub(crate) fn rule(self, budget: &mut usize) -> Option<Vec<Definition>> {
        if self.pattern.len() == 1 {
            let name = self.source[self.span.clone()].to_owned();
            *budget = budget.checked_sub(name.len())?;
            let input = self.pattern.into_iter().next()?;
            return Some(vec![Definition {
                name,
                input,
                output: self.output,
            }]);
        }
        let mut rule = Vec::new();
        for (index, input) in self.pattern.iter().enumerate() {
            for output in self.target(index) {
                let definition = Definition {
                    name: crate::text::rule(input, &output),
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
