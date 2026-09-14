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
    for group in &partition {
        for &index in &order[group.clone()] {
            color[index] = group.start;
        }
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
    let incoming = crate::graph::Graph::new(
        color.len(),
        edge.iter()
            .enumerate()
            .flat_map(|(source, adjacent)| {
                adjacent
                    .iter()
                    .map(move |&(kind, target)| (target, source, kind))
            })
            .collect(),
    );
    let mut affected = vec![true; color.len()];
    let mut changed = Vec::new();
    let mut boundary = Vec::new();
    loop {
        boundary.clear();
        changed.clear();
        for group in &partition {
            if !order[group.clone()].iter().any(|&index| affected[index]) {
                boundary.push(group.clone());
                continue;
            }
            let begin = boundary.len();
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
            }
            boundary.push(start..group.end);
            if boundary.len() > begin + 1 {
                for part in &boundary[begin..] {
                    for &index in &order[part.clone()] {
                        if color[index] != part.start {
                            changed.push((index, part.start));
                        }
                    }
                }
            }
        }
        if boundary.len() == partition.len() {
            for (ordinal, group) in boundary.iter().enumerate() {
                for &index in &order[group.clone()] {
                    color[index] = ordinal;
                }
            }
            return color;
        }
        affected.fill(false);
        for &(index, value) in &changed {
            color[index] = value;
            for &(_, source) in &incoming[index] {
                affected[source] = true;
            }
        }
        std::mem::swap(&mut partition, &mut boundary);
    }
}

#[cfg(test)]
#[path = "test/partition.rs"]
mod test;
