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
        let space = self.space()?;
        budget.set(budget.get().checked_sub(space)?);
        Some(
            (0..self.pattern.len())
                .map(|entered| Definition {
                    name: self.name(entered),
                    input: self.pattern[entered].input.clone(),
                    rest: self
                        .other(entered)
                        .map(|index| self.pattern[index].input.clone())
                        .collect(),
                    output: self.output.clone(),
                })
                .collect(),
        )
    }

    fn space(&self) -> Option<usize> {
        let count = self.pattern.len();
        if count == 1 {
            return Some(0);
        }
        let subset = 1usize.checked_shl(u32::try_from(count - 1).ok()?)?;
        let input = self
            .pattern
            .iter()
            .map(|pattern| 1 + crate::size::input(&pattern.input))
            .sum::<usize>()
            .checked_mul(subset)?;
        let output = (subset - 1)
            .checked_add((count - 1).checked_mul(subset / 2)?)?
            .checked_add(crate::size::output(&self.output))?
            .checked_mul(count)?;
        input.checked_add(output)
    }

    fn other(&self, entered: usize) -> impl Iterator<Item = usize> {
        (0..self.pattern.len()).filter(move |&index| index != entered)
    }

    fn name(&self, entered: usize) -> String {
        if self.pattern.len() == 1 {
            return self.source[self.span.clone()].to_owned();
        }
        std::iter::once(entered)
            .chain(self.other(entered))
            .map(|index| self.pattern[index].span.clone())
            .chain(self.sink.clone())
            .map(|span| &self.source[span])
            .collect::<Vec<_>>()
            .join(" ")
    }
}
