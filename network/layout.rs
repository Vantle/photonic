use crate::matrix::Span;
use std::ops::Range;

#[derive(Clone, Debug, Default)]
pub struct Layout {
    total: usize,
    decay: Vec<Range<usize>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    Weight,
    Bias,
    Scale,
    Table,
}

impl Layout {
    pub fn allocate(&mut self, row: usize, column: usize, kind: Kind) -> Span {
        let span = Span {
            start: self.total,
            row,
            column,
        };
        self.total += row * column;
        if kind == Kind::Weight {
            self.decay.push(span.start..span.end());
        }
        span
    }

    pub fn total(&self) -> usize {
        self.total
    }

    pub fn decay(&self) -> &[Range<usize>] {
        &self.decay
    }
}
