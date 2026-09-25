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
