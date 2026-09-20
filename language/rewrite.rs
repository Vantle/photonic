use crate::change::Change;
use crate::flow::{Binding, Place};
use crate::layout::Layout;
use crate::recipe::Recipe;
use crate::state::{Frame, State, Token, World};
use std::collections::BTreeMap;

pub(crate) struct Result {
    pub state: State,
    pub change: Change,
    pub layout: Layout,
}

fn remainder(source: &State, binding: &Binding, selected: &crate::basis::Set<Place>) -> Vec<Token> {
    let mut value = BTreeMap::new();
    for &world in &binding.world {
        for token in &source.world[world].particle {
            if selected.iter().any(|place| match place {
                Place::World(_, id) | Place::Held(_, id) => *id == token.id,
            }) {
                continue;
            }
            value.entry(token.id).or_insert_with(|| token.clone());
        }
    }
    value.into_values().collect()
}

pub(crate) fn apply(
    source: &State,
    frame: usize,
    owner: usize,
    recipe: &Recipe,
    binding: &Binding,
    layout: &Layout,
) -> Result {
    let returning = owner == frame && frame != 0;
    let parent = if returning {
        source.frame[frame].parent.unwrap()
    } else {
        frame
    };
    let mut state = State {
        world: source.world.clone(),
        frame: source.frame.clone(),
    };
    for &world in binding.world.iter().rev() {
        state.world.remove(world);
    }
    let start = state.world.len();
    let mut change = Change {
        world: binding.world.clone(),
        insertion: start..start + recipe.output.len(),
        frame: Vec::new(),
    };
    let remainder = remainder(source, binding, &binding.footprint);
    let enclosed = (recipe.nested && binding.exact != binding.footprint)
        .then(|| self::remainder(source, binding, &binding.exact));
    let mut reserve = BTreeMap::new();
    if recipe.nested {
        for place in &binding.exact {
            let token = match *place {
                Place::World(world, id) => source.world[world]
                    .particle
                    .iter()
                    .find(|token| token.id == id),
                Place::Held(frame, id) => {
                    source.frame[frame].held.iter().find(|token| token.id == id)
                }
            }
            .unwrap();
            reserve.entry(token.id).or_insert_with(|| token.clone());
        }
        if returning {
            for token in &source.frame[frame].held {
                reserve.entry(token.id).or_insert_with(|| token.clone());
            }
        }
    }
    let reserve = reserve.into_values().collect::<Vec<_>>();
    let mut vacant = if recipe.nested {
        (1..source.frame.len())
            .rev()
            .filter(|index| layout.reach.frame.binary_search(index).is_err())
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let mut next = layout.resource;
    for output in &recipe.output {
        let target = if let Some(scope) = output.scope {
            let target = vacant.pop().unwrap_or(state.frame.len());
            let value = Frame {
                scope,
                parent: Some(parent),
                lexical: Some(owner),
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
        let mut particle = if output.scope.is_some() {
            enclosed.as_ref().unwrap_or(&remainder).clone()
        } else {
            remainder.clone()
        };
        particle.reserve(output.emission.len());
        for emission in &output.emission {
            particle.push(Token {
                id: next,
                value: emission.value,
                capture: emission.capture.then_some(owner),
            });
            next += 1;
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
