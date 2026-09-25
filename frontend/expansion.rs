use std::cell::Cell;

use crate::source::Value;

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
        .map(|value| crate::size::particle(value))
        .sum::<usize>()
        .checked_mul(right.len())?
        .checked_add(
            right
                .iter()
                .map(|value| crate::size::particle(value))
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
