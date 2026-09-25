use std::cell::Cell;
use std::ops::Range;

use crate::source::{Definition, Output, Value};

pub(crate) struct Pattern {
    pub input: Vec<Vec<Value>>,
    pub span: Range<usize>,
}

pub(crate) struct Partition<'source> {
    pub source: &'source str,
    pub span: Range<usize>,
    pub pattern: Vec<Pattern>,
    pub sink: Option<Range<usize>>,
    pub output: Vec<Output>,
}

impl Partition<'_> {
    pub(crate) fn rule(self, budget: &Cell<usize>) -> Option<Vec<Definition>> {
        if self.pattern.len() == 1 {
            let name = self.name(0, &[]);
            let input = self.pattern.into_iter().next()?.input;
            return Some(vec![Definition {
                name,
                input,
                output: self.output,
            }]);
        }
        let every = (0..self.pattern.len()).collect::<Vec<_>>();
        every
            .iter()
            .map(|&entered| self.definition(entered, &every, budget))
            .collect()
    }

    fn definition(
        &self,
        entered: usize,
        remaining: &[usize],
        budget: &Cell<usize>,
    ) -> Option<Definition> {
        let rest = remaining
            .iter()
            .copied()
            .filter(|&index| index != entered)
            .collect::<Vec<_>>();
        let output = if rest.is_empty() {
            charge(budget, crate::size::output(&self.output))?;
            self.output.clone()
        } else {
            vec![Output {
                particle: rest
                    .iter()
                    .map(|&next| {
                        Some(Value::Rule {
                            rule: Box::new(self.definition(next, &rest, budget)?),
                        })
                    })
                    .collect::<Option<_>>()?,
                body: None,
            }]
        };
        let name = self.name(entered, &rest);
        let input = &self.pattern[entered].input;
        charge(budget, 1 + name.len() + crate::size::input(input))?;
        Some(Definition {
            name,
            input: input.clone(),
            output,
        })
    }

    fn name(&self, entered: usize, rest: &[usize]) -> String {
        if self.pattern.len() == 1 {
            return self.source[self.span.clone()].to_owned();
        }
        std::iter::once(entered)
            .chain(rest.iter().copied())
            .map(|index| self.pattern[index].span.clone())
            .chain(self.sink.clone())
            .map(|span| &self.source[span])
            .collect::<Vec<_>>()
            .join(" ")
    }
}

fn charge(budget: &Cell<usize>, size: usize) -> Option<()> {
    budget.set(budget.get().checked_sub(size)?);
    Some(())
}
