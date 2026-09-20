use super::space::Space;
use crate::index::Index;

#[derive(Clone, Copy)]
pub(super) enum Strategy {
    Shared,
    Partitioned(usize),
}

pub(super) struct Request<'a> {
    pub space: &'a Space,
    pub order: &'a [usize],
    pub changed: &'a [usize],
    pub index: &'a Index,
    pub depth: Option<usize>,
}

pub(super) fn select(request: Request<'_>) -> Option<Strategy> {
    let prefix = &request.order[..request.order.len().saturating_sub(1)];
    let Some(depth) = prefix
        .iter()
        .rposition(|position| request.changed.contains(position))
    else {
        return Some(Strategy::Shared);
    };
    if request.order.len() < 3 {
        return None;
    }
    for &position in prefix
        .iter()
        .filter(|position| request.changed.contains(position))
    {
        let domain = &request.space.domain[position];
        if domain.is_empty()
            || (domain.len() <= request.index.insertion.len()
                && request.index.insertion.contains(&domain[0].site))
        {
            return None;
        }
    }
    let depth = request.depth.map_or(depth, |previous| previous.max(depth));
    let position = request.order[depth];
    if depth + 1 == prefix.len() && request.space.pattern[position].len() < 8 {
        return None;
    }
    (request.space.domain[position].len() > 1).then_some(Strategy::Partitioned(depth))
}
