use super::space::Space;
use crate::index::Index;

#[derive(Clone, Copy)]
pub(super) enum Strategy {
    Shared,
    Partitioned(usize),
    Layered { previous: usize, depth: usize },
}

#[derive(Clone, Copy)]
pub(super) enum History {
    Single {
        depth: usize,
        granularity: Option<usize>,
    },
    Nested(usize),
}

impl History {
    fn depth(self) -> usize {
        match self {
            Self::Single { depth, .. } | Self::Nested(depth) => depth,
        }
    }

    fn layered(self) -> bool {
        match self {
            Self::Single { granularity, .. } => granularity.is_some_and(|length| length <= 256),
            Self::Nested(_) => true,
        }
    }
}

pub(super) struct Request<'request> {
    pub space: &'request Space,
    pub order: &'request [usize],
    pub changed: &'request [usize],
    pub index: &'request Index,
    pub history: Option<History>,
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
            || (domain.len() <= request.index.delta().insertion.len()
                && request.index.delta().insertion.contains(&domain[0].site))
        {
            return None;
        }
    }
    let changed = depth;
    let depth = request
        .history
        .map_or(depth, |previous| previous.depth().max(depth));
    let position = request.order[depth];
    if depth + 1 == prefix.len() && !crate::particle::wide(request.space.query.width(position)) {
        return None;
    }
    if request.space.domain[position].len() <= 1 {
        return None;
    }
    if let Some(previous) = request.history
        && previous.depth() != changed
        && previous.layered()
    {
        return Some(Strategy::Layered {
            previous: previous.depth(),
            depth: changed,
        });
    }
    Some(Strategy::Partitioned(depth))
}
