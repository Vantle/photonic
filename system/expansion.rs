use std::cell::Cell;

use crate::source::{Definition, Value};

fn definition(value: &Definition) -> usize {
    1 + value.name.len()
        + value
            .input
            .iter()
            .map(|value| particle(value))
            .sum::<usize>()
        + value
            .output
            .iter()
            .map(|value| {
                1 + particle(&value.particle)
                    + value.body.iter().flatten().map(definition).sum::<usize>()
            })
            .sum::<usize>()
}

fn particle(value: &[Value]) -> usize {
    1 + value
        .iter()
        .map(|value| match value {
            Value::Atom(value) => 1 + value.len(),
            Value::Rule { rule } => definition(rule),
        })
        .sum::<usize>()
}

pub(crate) fn combine(
    left: Vec<Vec<Value>>,
    right: Vec<Vec<Value>>,
    budget: &Cell<usize>,
) -> Option<Vec<Vec<Value>>> {
    if left.len() == 1 && right.len() == 1 {
        let mut result = left.into_iter().next()?;
        result.extend(right.into_iter().next()?);
        return Some(vec![result]);
    }
    let size = left
        .iter()
        .map(|value| particle(value))
        .sum::<usize>()
        .checked_mul(right.len())?
        .checked_add(
            right
                .iter()
                .map(|value| particle(value))
                .sum::<usize>()
                .checked_mul(left.len())?,
        )?;
    budget.set(budget.get().checked_sub(size)?);
    Some(
        left.iter()
            .flat_map(|left| {
                right
                    .iter()
                    .map(move |right| left.iter().chain(right).cloned().collect())
            })
            .collect(),
    )
}
