use crate::change::Change;
use crate::flow::{Binding, Place};
use crate::layout::Layout;
use crate::program::{Instruction, Scope, Symbol};
use crate::state::{Frame, State, Token, World};
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

fn remainder(source: &State, binding: &Binding, selected: &crate::basis::Set<Place>) -> Vec<Token> {
    let mut value = smallvec::SmallVec::<[&Token; 8]>::new();
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
    value.into_iter().cloned().collect()
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
    let consumed = crate::consumption::Selection::new(binding);
    for &world in binding.world.iter().rev() {
        state.world.remove(world);
    }
    let start = state.world.len();
    let mut change = Change {
        world: binding.world.clone(),
        insertion: start..start + rule.output.len(),
        frame: (source.frame.len()..state.frame.len()).collect(),
    };
    for (index, frame) in source.frame.iter().enumerate() {
        if let Some(frame) = consumed.frame(index, frame) {
            state.frame[index] = frame;
            change.frame.push(index);
        }
    }
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
    let mut vacant = if nested {
        (1..source.frame.len())
            .rev()
            .filter(|index| layout.reach.frame.binary_search(index).is_err())
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let mut next = next;
    for output in &rule.output {
        let target = if let Some(body) = output.body {
            let target = vacant.pop().unwrap_or(state.frame.len());
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
        let mut particle = if output.body.is_some() {
            enclosed.as_ref().unwrap_or(&remainder).clone()
        } else {
            remainder.clone()
        };
        particle.reserve(output.particle.len());
        for &value in &output.particle {
            particle.push(Token::new(value, owner, &mut next));
        }
        state.world.push(
            World {
                frame: target,
                particle,
            }
            .into(),
        );
    }
    let reach = layout.reach.advance(source, &state, &change);
    change.frame.extend(
        layout
            .reach
            .frame
            .iter()
            .copied()
            .filter(|index| reach.frame.binary_search(index).is_err()),
    );
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
