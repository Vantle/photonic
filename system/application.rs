use crate::basis::Set;
use crate::flow::{Applied, Binding, Closure, Flow, Place};
use crate::program::{Instruction, Symbol};
use crate::state::{Frame, State, Token, World};
use std::collections::{BTreeMap, BTreeSet, HashMap};

struct Draft {
    resource: Vec<(Place, Set<Place>)>,
    context: Vec<Set<usize>>,
    frame: Vec<Option<usize>>,
}

struct Import<'state> {
    closure: Closure<'state>,
    frame: HashMap<usize, usize>,
    resource: HashMap<usize, usize>,
    next: usize,
}

impl Import<'_> {
    fn include(&mut self, index: usize, state: &mut State, flow: &mut Draft) -> usize {
        if let Some(index) = self.closure.flow.frame[index] {
            return index;
        }
        if let Some(&index) = self.frame.get(&index) {
            return index;
        }
        let position = state.frame.len();
        self.frame.insert(index, position);
        let original = self.closure.state.frame[index].clone();
        state.frame.push(Frame {
            scope: original.scope,
            parent: None,
            lexical: None,
            held: Vec::new(),
        });
        flow.frame.push(None);
        let parent = original
            .parent
            .map(|index| self.include(index, state, flow));
        let lexical = original
            .lexical
            .map(|index| self.include(index, state, flow));
        let mut held = Vec::new();
        for token in original.held {
            let id = *self.resource.entry(token.id).or_insert_with(|| {
                let next = self.next;
                self.next += 1;
                next
            });
            let capture = token.capture.map(|index| self.include(index, state, flow));
            held.push(Token {
                id,
                value: token.value,
                capture,
            });
            flow.resource.push((
                Place::Held(position, id),
                self.closure.flow.resource[&Place::Held(index, token.id)].clone(),
            ));
        }
        state.frame[position] = Frame {
            scope: original.scope,
            parent,
            lexical,
            held,
        };
        position
    }
}

pub(crate) fn apply(
    source: &State,
    frame: usize,
    owner: Option<usize>,
    rule: &Instruction,
    binding: &Binding,
    closure: Option<Closure<'_>>,
) -> Applied {
    let mut state = State {
        world: Vec::new(),
        frame: source.frame.clone(),
    };
    let mut flow = Draft {
        resource: source
            .frame
            .iter()
            .enumerate()
            .flat_map(|(index, frame)| {
                frame.held.iter().map(move |token| {
                    let place = Place::Held(index, token.id);
                    (place, Set::single(place))
                })
            })
            .collect(),
        context: Vec::new(),
        frame: (0..source.frame.len()).map(Some).collect(),
    };
    let mut next = source
        .world
        .iter()
        .flat_map(|world| &world.particle)
        .chain(source.frame.iter().flat_map(|frame| &frame.held))
        .map(|token| token.id)
        .max()
        .map_or(0, |id| id + 1);
    let owner = if let Some(closure) = closure {
        let mut resource = HashMap::new();
        for (index, frame) in closure.state.frame.iter().enumerate() {
            for token in &frame.held {
                let basis = &closure.flow.resource[&Place::Held(index, token.id)];
                if basis.len() != 1
                    || token
                        .capture
                        .is_some_and(|frame| closure.flow.frame[frame].is_none())
                {
                    continue;
                }
                let original = match *basis.first().unwrap() {
                    Place::World(index, id) => source.world[index]
                        .particle
                        .iter()
                        .find(|token| token.id == id),
                    Place::Held(index, id) => {
                        source.frame[index].held.iter().find(|token| token.id == id)
                    }
                };
                if let Some(original) = original {
                    let capture = token.capture.and_then(|frame| closure.flow.frame[frame]);
                    if original.value == token.value && original.capture == capture {
                        resource.insert(token.id, original.id);
                    }
                }
            }
        }
        let capture = closure.capture;
        let mut import = Import {
            closure,
            frame: HashMap::new(),
            resource,
            next,
        };
        let owner = import.include(capture, &mut state, &mut flow);
        next = import.next;
        owner
    } else {
        owner.expect("lexical rule has an owner")
    };
    let returning = owner == frame && frame != 0;
    let parent = if returning {
        source.frame[frame].parent.unwrap()
    } else {
        frame
    };
    for (index, world) in source.world.iter().enumerate() {
        if binding.world.contains(&index) {
            continue;
        }
        let target = state.world.len();
        state.world.push(world.clone());
        flow.context.push(Set::single(index));
        for token in &world.particle {
            flow.resource.push((
                Place::World(target, token.id),
                Set::single(Place::World(index, token.id)),
            ));
        }
    }
    let consumed = if returning {
        source.frame[frame]
            .held
            .iter()
            .map(|token| Place::Held(frame, token.id))
            .collect::<BTreeSet<_>>()
    } else {
        BTreeSet::new()
    };
    let basis = binding
        .footprint
        .union(&consumed)
        .copied()
        .collect::<Set<_>>();
    for output in &rule.output {
        let selected = if output.body.is_some() {
            &binding.exact
        } else {
            &binding.footprint
        };
        let removed = selected
            .iter()
            .map(|place| match place {
                Place::World(_, id) | Place::Held(_, id) => *id,
            })
            .collect::<BTreeSet<_>>();
        let mut remainder = BTreeMap::<usize, (Token, BTreeSet<Place>)>::new();
        for &index in &binding.world {
            for token in &source.world[index].particle {
                if removed.contains(&token.id) {
                    continue;
                }
                remainder
                    .entry(token.id)
                    .or_insert_with(|| (token.clone(), BTreeSet::new()))
                    .1
                    .insert(Place::World(index, token.id));
            }
        }
        let target = if let Some(scope) = output.body {
            let target = state.frame.len();
            let mut reserve = BTreeMap::<usize, (Token, BTreeSet<Place>)>::new();
            for &place in binding.exact.union(&consumed) {
                let token = match place {
                    Place::World(index, id) => source.world[index]
                        .particle
                        .iter()
                        .find(|token| token.id == id),
                    Place::Held(index, id) => {
                        source.frame[index].held.iter().find(|token| token.id == id)
                    }
                }
                .unwrap();
                reserve
                    .entry(token.id)
                    .or_insert_with(|| (token.clone(), BTreeSet::new()))
                    .1
                    .insert(place);
            }
            state.frame.push(Frame {
                scope,
                parent: Some(parent),
                lexical: Some(owner),
                held: reserve.values().map(|(token, _)| token.clone()).collect(),
            });
            flow.frame.push(None);
            for (id, (_, basis)) in reserve {
                flow.resource.push((Place::Held(target, id), basis.into()));
            }
            target
        } else {
            parent
        };
        let index = state.world.len();
        let mut particle = Vec::new();
        for (_, (token, basis)) in remainder {
            flow.resource
                .push((Place::World(index, token.id), basis.into()));
            particle.push(token);
        }
        for &value in &output.particle {
            let token = Token {
                id: next,
                value,
                capture: matches!(value, Symbol::Rule(_)).then_some(owner),
            };
            next += 1;
            flow.resource
                .push((Place::World(index, token.id), basis.clone()));
            particle.push(token);
        }
        state.world.push(World {
            frame: target,
            particle,
        });
        flow.context.push(binding.world.iter().copied().collect());
    }
    Applied {
        state,
        flow: Flow {
            resource: flow.resource.into_iter().collect(),
            context: flow.context,
            frame: flow.frame,
        },
    }
}
