pub(crate) fn classify<Key: Ord>(value: &[Key]) -> Vec<usize> {
    let mut order = (0..value.len()).collect::<Vec<_>>();
    order.sort_unstable_by(|&left, &right| value[left].cmp(&value[right]));
    let mut result = vec![0; value.len()];
    let mut ordinal = 0;
    for pair in order.windows(2) {
        if value[pair[0]] != value[pair[1]] {
            ordinal += 1;
        }
        result[pair[1]] = ordinal;
    }
    result
}

pub(crate) fn refine(edge: &crate::graph::Graph, mut color: Vec<usize>) -> Vec<usize> {
    let mut order = (0..color.len()).collect::<Vec<_>>();
    order.sort_unstable_by_key(|&index| color[index]);
    let mut partition = Vec::new();
    let mut start = 0;
    for position in 0..order.len() {
        if position > 0 && color[order[position - 1]] != color[order[position]] {
            partition.push(start..position);
            start = position;
        }
    }
    if start < order.len() {
        partition.push(start..order.len());
    }
    let mut length = 0;
    let range = edge
        .iter()
        .map(|adjacent| {
            let start = length;
            length += adjacent.len();
            start..length
        })
        .collect::<Vec<_>>();
    let mut signature = vec![(0, 0); length];
    let mut next = vec![0; color.len()];
    let mut boundary = Vec::new();
    loop {
        boundary.clear();
        for group in &partition {
            if group.len() > 1 {
                for &index in &order[group.clone()] {
                    let signature = &mut signature[range[index].clone()];
                    for (value, &(kind, target)) in signature.iter_mut().zip(&edge[index]) {
                        *value = (kind, color[target]);
                    }
                    signature.sort_unstable();
                }
                order[group.clone()].sort_unstable_by(|&left, &right| {
                    signature[range[left].clone()].cmp(&signature[range[right].clone()])
                });
            }
            let mut start = group.start;
            for position in group.clone() {
                if position > group.start
                    && signature[range[order[position - 1]].clone()]
                        != signature[range[order[position]].clone()]
                {
                    boundary.push(start..position);
                    start = position;
                }
                next[order[position]] = boundary.len();
            }
            boundary.push(start..group.end);
        }
        if boundary.len() == partition.len() {
            return next;
        }
        std::mem::swap(&mut partition, &mut boundary);
        std::mem::swap(&mut color, &mut next);
    }
}

#[cfg(test)]
#[path = "test/partition.rs"]
mod test;
