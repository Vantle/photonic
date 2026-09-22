use crate::change::Change;
use crate::flow::{Binding, Place};
use crate::layout::Layout;
use crate::program::{Instruction, Scope, Symbol};
use crate::state::{Frame, State, Token, World};
use smallvec::SmallVec;
use std::collections::BTreeMap;

pub(crate) struct Result {
    pub state: State,
    pub change: Change,
    pub layout: Layout,
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
    #[cfg(feature = "measurement")]
    let _measurement =
        crate::measurement::profile::Scope::new(crate::measurement::profile::Phase::Rewrite);
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
    let nested = rule.output.iter().any(|output| output.body.is_some());
    for &world in binding.world.iter().rev() {
        state.world.remove(world);
    }
    let start = state.world.len();
    let mut change = Change {
        world: binding.world.clone(),
        insertion: start..start + rule.output.len(),
        frame: (source.frame.len()..state.frame.len()).collect(),
    };
    change
        .frame
        .extend(crate::consumption::apply(&mut state, binding));
    let remainder = remainder(source, binding, &binding.footprint);
    let enclosed = (nested && binding.exact != binding.footprint)
        .then(|| self::remainder(source, binding, &binding.exact));
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
    let mut vacant =
        (1..source.frame.len()).filter(|index| layout.reach.frame.binary_search(index).is_err());
    let mut next = next;
    for output in &rule.output {
        let target = if let Some(body) = output.body {
            let target = vacant.next().unwrap_or(state.frame.len());
            let value = Frame {
                scope: body,
                parent: Some(parent),
                lexical: Some(owner),
                particle: scope[body]
                    .rule
                    .iter()
                    .map(|&rule| Token::new(Symbol::Rule(rule), target, &mut next))
                    .collect(),
                held: reserve.clone(),
            }
            .into();
            if target == state.frame.len() {
                state.frame.push(value);
            } else {
                state.frame[target] = value;
            }
            change.frame.push(target);
            target
        } else {
            parent
        };
        let base = match output.body {
            Some(_) => enclosed.as_ref().unwrap_or(&remainder),
            None => &remainder,
        };
        let particle = base
            .iter()
            .copied()
            .cloned()
            .chain(
                output
                    .particle
                    .iter()
                    .map(|&value| Token::new(value, owner, &mut next)),
            )
            .collect();
        state.world.push(
            World {
                frame: target,
                particle,
            }
            .into(),
        );
    }
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
    }
}

#[cfg(test)]
#[path = "test/oracle.rs"]
mod test;
