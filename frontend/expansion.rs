use crate::source::Value;

// A join builds one particle for every choice of one particle from each factor. What it builds is
// charged once, before it is built, so whether a join fits depends on what it makes and never on
// the order of its factors.
pub(crate) fn product(factor: Vec<Vec<Vec<Value>>>, budget: &mut usize) -> Option<Vec<Vec<Value>>> {
    let count = factor
        .iter()
        .try_fold(1_usize, |count, choice| count.checked_mul(choice.len()))?;
    if count == 0 {
        return Some(Vec::new());
    }
    if count == 1 {
        let particle = factor.into_iter().flatten().reduce(|mut particle, next| {
            particle.extend(next);
            particle
        });
        return Some(vec![particle.unwrap_or_default()]);
    }
    *budget = budget.checked_sub(size(&factor, count)?)?;
    let mut result = Vec::with_capacity(count);
    let mut chosen = vec![0; factor.len()];
    loop {
        let mut particle = Vec::with_capacity(
            chosen
                .iter()
                .zip(&factor)
                .map(|(&index, choice)| choice[index].len())
                .sum(),
        );
        for (&index, choice) in chosen.iter().zip(&factor) {
            particle.extend(choice[index].iter().cloned());
        }
        result.push(particle);
        let Some(position) =
            (0..factor.len()).rfind(|&position| chosen[position] + 1 < factor[position].len())
        else {
            return Some(result);
        };
        chosen[position] += 1;
        chosen[position + 1..].fill(0);
    }
}

fn size(factor: &[Vec<Vec<Value>>], count: usize) -> Option<usize> {
    factor.iter().try_fold(count, |size, choice| {
        choice
            .iter()
            .map(|particle| crate::size::particle(particle) - 1)
            .sum::<usize>()
            .checked_mul(count / choice.len())?
            .checked_add(size)
    })
}
