use crate::configuration::Configuration;
use crate::particle::Particle;

const EXACT: usize = 10;

pub fn distance(left: &Configuration, right: &Configuration) -> f64 {
    let (large, small) = if (left.coherence().len(), left) < (right.coherence().len(), right) {
        (right.coherence(), left.coherence())
    } else {
        (left.coherence(), right.coherence())
    };
    let total = large
        .iter()
        .chain(small)
        .map(|particle| particle.len() + 1)
        .sum::<usize>();
    if total == 0 {
        return 0.0;
    }
    let cost = if small.len() <= EXACT {
        exact(large, small)
    } else {
        greedy(large, small)
    };
    cost as f64 / total as f64
}

fn exact(left: &[Particle], right: &[Particle]) -> usize {
    let full = 1usize << right.len();
    let mut cost = vec![usize::MAX; full];
    cost[0] = 0;
    for particle in left {
        let mut next = vec![usize::MAX; full];
        for mask in 0..full {
            if cost[mask] == usize::MAX {
                continue;
            }
            next[mask] = next[mask].min(cost[mask] + particle.len() + 1);
            for (index, other) in right.iter().enumerate() {
                if mask & (1 << index) != 0 {
                    continue;
                }
                let value = cost[mask] + particle.difference(other);
                next[mask | (1 << index)] = next[mask | (1 << index)].min(value);
            }
        }
        cost = next;
    }
    (0..full)
        .filter(|&mask| cost[mask] != usize::MAX)
        .map(|mask| {
            cost[mask]
                + right
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| mask & (1 << index) == 0)
                    .map(|(_, particle)| particle.len() + 1)
                    .sum::<usize>()
        })
        .min()
        .unwrap_or(0)
}

fn greedy(left: &[Particle], right: &[Particle]) -> usize {
    let mut open = vec![true; right.len()];
    let mut cost = 0;
    for particle in left {
        let best = right
            .iter()
            .enumerate()
            .filter(|(index, _)| open[*index])
            .map(|(index, other)| (particle.difference(other), index))
            .min();
        match best {
            Some((value, index)) if value <= particle.len() + 1 + right[index].len() + 1 => {
                open[index] = false;
                cost += value;
            }
            _ => cost += particle.len() + 1,
        }
    }
    cost + right
        .iter()
        .zip(&open)
        .filter(|(_, open)| **open)
        .map(|(particle, _)| particle.len() + 1)
        .sum::<usize>()
}
