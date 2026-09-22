pub(crate) fn classify<Key: Ord>(value: &[Key]) -> Vec<usize> {
    match value {
        [] => return Vec::new(),
        [_] => return vec![0],
        [left, right] => {
            return match left.cmp(right) {
                std::cmp::Ordering::Less => vec![0, 1],
                std::cmp::Ordering::Equal => vec![0, 0],
                std::cmp::Ordering::Greater => vec![1, 0],
            };
        }
        _ => {}
    }
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
    let mut boundary = vec![0; color.len()];
    let mut pending = Vec::new();
    let mut start = 0;
    for end in 1..=order.len() {
        if end < order.len() && color[order[end - 1]] == color[order[end]] {
            continue;
        }
        boundary[start] = end;
        if end - start > 1 {
            pending.push(start);
        }
        start = end;
    }
    let mut start = 0;
    while start < order.len() {
        let end = boundary[start];
        for &index in &order[start..end] {
            color[index] = start;
        }
        start = end;
    }
    if pending.is_empty() {
        return color;
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
        edge.iter().enumerate().flat_map(|(source, adjacent)| {
            adjacent
                .iter()
                .map(move |&(kind, target)| (target, source, kind))
        }),
    );
    let mut scheduled = vec![false; color.len()];
    let mut changed = Vec::new();
    loop {
        changed.clear();
        for start in pending.drain(..) {
            scheduled[start] = false;
            let end = boundary[start];
            for &index in &order[start..end] {
                let signature = &mut signature[range[index].clone()];
                for (value, &(kind, target)) in signature.iter_mut().zip(&edge[index]) {
                    *value = (kind, color[target]);
                }
                signature.sort_unstable();
            }
            order[start..end].sort_unstable_by(|&left, &right| {
                signature[range[left].clone()].cmp(&signature[range[right].clone()])
            });
            let mut partition = start;
            for position in start + 1..end {
                if signature[range[order[position - 1]].clone()]
                    != signature[range[order[position]].clone()]
                {
                    boundary[partition] = position;
                    partition = position;
                }
                if partition != start {
                    changed.push((order[position], partition));
                }
            }
            boundary[partition] = end;
        }
        if changed.is_empty() {
            let mut start = 0;
            let mut ordinal = 0;
            while start < order.len() {
                let end = boundary[start];
                for &index in &order[start..end] {
                    color[index] = ordinal;
                }
                ordinal += 1;
                start = end;
            }
            return color;
        }
        for &(index, value) in &changed {
            color[index] = value;
        }
        for &(index, _) in &changed {
            for &(_, source) in &incoming[index] {
                let start = color[source];
                if boundary[start] - start > 1 && !std::mem::replace(&mut scheduled[start], true) {
                    pending.push(start);
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "test/partition.rs"]
mod test;
