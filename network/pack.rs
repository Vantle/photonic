use crate::input::{Input, Pointer};
use std::ops::Range;

#[derive(Clone, Debug, Default)]
pub struct Pack {
    pub feature: Vec<u16>,
    pub boundary: Vec<Range<usize>>,
    pub pointer: Vec<Vec<Pointer>>,
}

impl Pack {
    pub fn new(input: &[&Input], field: usize) -> Self {
        let mut pack = Self::default();
        for input in input {
            let start = pack.feature.len() / field;
            let length = input.length(field);
            let offset = start as u32;
            pack.feature.extend_from_slice(&input.feature);
            pack.boundary.push(start..start + length);
            pack.pointer.push(
                input
                    .pointer
                    .iter()
                    .map(|pointer| match *pointer {
                        Pointer::Unary { head, token } => Pointer::Unary {
                            head,
                            token: token + offset,
                        },
                        Pointer::Binary { head, left, right } => Pointer::Binary {
                            head,
                            left: left + offset,
                            right: right + offset,
                        },
                    })
                    .collect(),
            );
        }
        pack
    }
}
