// Each row takes at most one column and each column at most one row, so that the costs of the
// pairs taken sum to the least, where a row left alone costs nothing. The search runs over the
// shorter side, so a long list of parts against a few places stays quadratic in the places.
pub fn pair(cost: &[Vec<i64>]) -> Vec<Option<usize>> {
    let row = cost.len();
    let column = cost.first().map_or(0, Vec::len);
    if row <= column {
        let padded = cost
            .iter()
            .map(|entry| {
                entry
                    .iter()
                    .copied()
                    .chain(std::iter::repeat_n(0, row))
                    .collect()
            })
            .collect::<Vec<Vec<i64>>>();
        return cheapest(&padded)
            .into_iter()
            .map(|choice| (choice < column).then_some(choice))
            .collect();
    }
    let turned = (0..column)
        .map(|target| {
            (0..row)
                .map(|source| cost[source][target])
                .chain(std::iter::repeat_n(0, column))
                .collect()
        })
        .collect::<Vec<Vec<i64>>>();
    let mut pairing = vec![None; row];
    for (target, choice) in cheapest(&turned).into_iter().enumerate() {
        if choice < row {
            pairing[choice] = Some(target);
        }
    }
    pairing
}

pub fn cheapest(cost: &[Vec<i64>]) -> Vec<usize> {
    let row = cost.len();
    let column = cost.first().map_or(0, Vec::len);
    if row == 0 || column < row {
        return Vec::new();
    }
    let mut potential = vec![0i64; row + 1];
    let mut price = vec![0i64; column + 1];
    let mut owner = vec![0usize; column + 1];
    let mut way = vec![0usize; column + 1];
    for start in 1..=row {
        owner[0] = start;
        let mut current = 0;
        let mut slack = vec![i64::MAX; column + 1];
        let mut used = vec![false; column + 1];
        loop {
            used[current] = true;
            let occupant = owner[current];
            let mut delta = i64::MAX;
            let mut next = 0;
            for candidate in 1..=column {
                if used[candidate] {
                    continue;
                }
                let reduced =
                    cost[occupant - 1][candidate - 1] - potential[occupant] - price[candidate];
                if reduced < slack[candidate] {
                    slack[candidate] = reduced;
                    way[candidate] = current;
                }
                if slack[candidate] < delta {
                    delta = slack[candidate];
                    next = candidate;
                }
            }
            for candidate in 0..=column {
                if used[candidate] {
                    potential[owner[candidate]] += delta;
                    price[candidate] -= delta;
                } else {
                    slack[candidate] -= delta;
                }
            }
            current = next;
            if owner[current] == 0 {
                break;
            }
        }
        while current != 0 {
            let previous = way[current];
            owner[current] = owner[previous];
            current = previous;
        }
    }
    let mut choice = vec![0; row];
    for candidate in 1..=column {
        if owner[candidate] != 0 {
            choice[owner[candidate] - 1] = candidate - 1;
        }
    }
    choice
}
