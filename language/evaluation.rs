use crate::change::Change;
use crate::flow::Binding;
use crate::layout::Layout;
use crate::opening::{Opening, world};
use crate::place::Place;
use crate::profile;
use crate::program::{Instruction, Output, Scope};
use crate::state::{State, Token};
use smallvec::SmallVec;
use std::collections::BTreeMap;

pub(crate) struct Result {
    pub state: State,
    pub change: Change,
    pub layout: Layout,
    pub opened: Vec<usize>,
}

pub(crate) struct Request<'source> {
    pub source: &'source State,
    pub frame: usize,
    pub owner: usize,
    pub rule: &'source Instruction,
    pub scope: &'source [Scope],
    pub state: State,
    pub next: usize,
    pub binding: &'source Binding,
    pub layout: &'source Layout,
}

fn remainder<'source>(
    source: &'source State,
    binding: &Binding,
    selected: &crate::basis::Set<Place>,
) -> SmallVec<[&'source Token; 8]> {
    let mut value = SmallVec::<[&Token; 8]>::new();
    for &world in &binding.world {
        for token in &source.world[world].particle {
            if selected.contains(&Place::World(world, token.id)) {
                continue;
            }
            value.push(token);
        }
    }
    value.sort_by_key(|token| token.id);
    value.dedup_by_key(|token| token.id);
    value
}

pub(crate) fn apply(request: Request<'_>) -> Result {
    let _scope = profile::Scope::new(profile::Phase::Rewrite);
    let Request {
        source,
        frame,
        owner,
        rule,
        scope,
        mut state,
        next,
        binding,
        layout,
    } = request;
    let returning = owner == frame && frame != 0;
    let parent = if returning {
        source.frame[frame].parent.unwrap()
    } else {
        frame
    };
    let nested = rule
        .output
        .iter()
        .any(|output| matches!(output, Output::Scope(_)));
    for &world in binding.world.iter().rev() {
        state.world.remove(world);
    }
    let start = state.world.len();
    let mut change = Change {
        world: binding.world.clone(),
        insertion: start..start,
        frame: (source.frame.len()..state.frame.len()).collect(),
    };
    change
        .frame
        .extend(crate::consumption::apply(&mut state, binding));
    let remainder = remainder(source, binding, &binding.footprint);
    let enclosed = (nested && binding.exact != binding.footprint)
        .then(|| self::remainder(source, binding, &binding.exact));
    let enclosed = enclosed.as_ref().unwrap_or(&remainder);
    let mut reserve = BTreeMap::new();
    if nested {
        for place in &binding.exact {
            let token = source.token(*place).unwrap();
            reserve.entry(token.id).or_insert_with(|| token.clone());
        }
        if returning {
            for token in &source.frame[frame].held {
                reserve.entry(token.id).or_insert_with(|| token.clone());
            }
        }
    }
    let reserve = reserve.into_values().collect::<Vec<_>>();
    let mut opening = Opening {
        scope,
        held: &reserve,
        base: enclosed,
        capture: owner,
        vacant: (1..source.frame.len())
            .filter(|index| layout.reach.frame.binary_search(index).is_err()),
        next,
        opened: Vec::new(),
    };
    for output in &rule.output {
        match output {
            Output::Particle(particle) => state.world.push(world(
                parent,
                &remainder,
                particle,
                owner,
                &mut opening.next,
            )),
            Output::Scope(body) => opening.apply(&mut state, *body, Some(parent), Some(owner)),
        }
    }
    let opened = opening.opened;
    change.insertion = start..state.world.len();
    change.frame.extend(&opened);
    let reach = layout.reach.advance(source, &state, &change);
    if !std::sync::Arc::ptr_eq(&layout.reach.frame, &reach.frame) {
        change.frame.extend(
            layout
                .reach
                .frame
                .iter()
                .copied()
                .filter(|index| reach.frame.binary_search(index).is_err()),
        );
    }
    change.frame.sort_unstable();
    change.frame.dedup();
    let state = state.reclaim(&reach.frame);
    let layout = layout.advance(source, &state, &change, reach);
    Result {
        state,
        change,
        layout,
        opened,
    }
}

#[cfg(test)]
#[path = "test/oracle.rs"]
mod test;
