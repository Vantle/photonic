use std::collections::VecDeque;

pub(crate) fn feasible(candidate: &[Vec<usize>]) -> bool {
    let count = candidate
        .iter()
        .flatten()
        .max()
        .map_or(0, |world| world + 1);
    if candidate.len() > count {
        return false;
    }
    let mut owner = vec![None::<usize>; count];
    let mut selected = vec![None; candidate.len()];
    for root in 0..candidate.len() {
        let mut pending = VecDeque::from([root]);
        let mut previous = vec![None; candidate.len()];
        let mut visited = vec![false; candidate.len()];
        visited[root] = true;
        let mut found = None;
        while let Some(position) = pending.pop_front() {
            for &world in &candidate[position] {
                let Some(next) = owner[world] else {
                    found = Some((position, world));
                    break;
                };
                if !visited[next] {
                    visited[next] = true;
                    previous[next] = Some(position);
                    pending.push_back(next);
                }
            }
            if found.is_some() {
                break;
            }
        }
        let Some((mut position, mut world)) = found else {
            return false;
        };
        loop {
            owner[world] = Some(position);
            let displaced = selected[position].replace(world);
            let Some(parent) = previous[position] else {
                break;
            };
            world = displaced.unwrap();
            position = parent;
        }
    }
    true
}
