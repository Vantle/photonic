use crate::program::{Instruction, Symbol};
use crate::state::{Frame, State, Token, World};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Place {
    World(usize, usize),
    Held(usize, usize),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Flow {
    pub resource: BTreeMap<Place, BTreeSet<Place>>,
    pub context: Vec<BTreeSet<usize>>,
    pub frame: Vec<Option<usize>>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Binding {
    pub world: BTreeSet<usize>,
    pub footprint: BTreeSet<Place>,
    pub exact: BTreeSet<Place>,
    pub read: BTreeSet<Place>,
}

pub struct Closure<'state> {
    pub state: &'state State,
    pub flow: &'state Flow,
    pub capture: usize,
}

pub struct Applied {
    pub state: State,
    pub flow: Flow,
}

impl Flow {
    pub fn identity(state: &State) -> Self {
        let mut resource = BTreeMap::new();
        for (world, value) in state.world.iter().enumerate() {
            for token in &value.particle {
                let place = Place::World(world, token.id);
                resource.insert(place, BTreeSet::from([place]));
            }
        }
        for (frame, value) in state.frame.iter().enumerate() {
            for token in &value.held {
                let place = Place::Held(frame, token.id);
                resource.insert(place, BTreeSet::from([place]));
            }
        }
        Self {
            resource,
            context: (0..state.world.len())
                .map(|index| BTreeSet::from([index]))
                .collect(),
            frame: (0..state.frame.len()).map(Some).collect(),
        }
    }

    pub fn compose(&self, event: &Flow) -> Self {
        Self {
            resource: event
                .resource
                .iter()
                .map(|(&place, source)| {
                    (
                        place,
                        source
                            .iter()
                            .flat_map(|key| self.resource[key].iter().copied())
                            .collect(),
                    )
                })
                .collect(),
            context: event
                .context
                .iter()
                .map(|value| {
                    value
                        .iter()
                        .flat_map(|&index| self.context[index].iter().copied())
                        .collect()
                })
                .collect(),
            frame: event
                .frame
                .iter()
                .map(|index| index.and_then(|index| self.frame[index]))
                .collect(),
        }
    }

    pub fn project(
        &self,
        source: &State,
        target: &State,
        selection: &[(usize, Vec<usize>)],
        frame: usize,
    ) -> Option<Binding> {
        let mut footprint = BTreeSet::new();
        let mut exact = BTreeSet::new();
        let mut world = BTreeSet::new();
        for (index, selected) in selection {
            world.extend(&self.context[*index]);
            for &id in selected {
                let token = target.world[*index]
                    .particle
                    .iter()
                    .find(|token| token.id == id)
                    .unwrap();
                let basis = &self.resource[&Place::World(*index, id)];
                footprint.extend(basis);
                if basis.len() != 1 {
                    continue;
                }
                if let Some(&Place::World(index, id)) = basis.first()
                    && source.world[index].particle.iter().any(|value| {
                        value.id == id
                            && value.value == token.value
                            && value.capture
                                == token.capture.and_then(|capture| self.frame[capture])
                    })
                {
                    exact.insert(Place::World(index, id));
                }
            }
        }
        for place in &footprint {
            match place {
                Place::World(index, _) => {
                    world.insert(*index);
                }
                Place::Held(_, _) => return None,
            }
        }
        if world
            .iter()
            .any(|&index| source.world[index].frame != frame)
        {
            return None;
        }
        Some(Binding {
            world,
            footprint,
            exact,
            read: BTreeSet::new(),
        })
    }
}

struct Import<'state> {
    closure: Closure<'state>,
    frame: HashMap<usize, usize>,
    resource: HashMap<usize, usize>,
    next: usize,
}

impl Import<'_> {
    fn include(&mut self, index: usize, state: &mut State, flow: &mut Flow) -> usize {
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
            flow.resource.insert(
                Place::Held(position, id),
                self.closure.flow.resource[&Place::Held(index, token.id)].clone(),
            );
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

pub fn apply(
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
    let mut flow = Flow::identity(source);
    flow.resource
        .retain(|place, _| matches!(place, Place::Held(_, _)));
    flow.context.clear();
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
        flow.context.push(BTreeSet::from([index]));
        for token in &world.particle {
            flow.resource.insert(
                Place::World(target, token.id),
                BTreeSet::from([Place::World(index, token.id)]),
            );
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
        .collect::<BTreeSet<_>>();
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
                flow.resource.insert(Place::Held(target, id), basis);
            }
            target
        } else {
            parent
        };
        let index = state.world.len();
        let mut particle = Vec::new();
        for (_, (token, basis)) in remainder {
            flow.resource.insert(Place::World(index, token.id), basis);
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
                .insert(Place::World(index, token.id), basis.clone());
            particle.push(token);
        }
        state.world.push(World {
            frame: target,
            particle,
        });
        flow.context.push(binding.world.clone());
    }
    Applied { state, flow }
}

impl Applied {
    pub fn canonical(self) -> Self {
        let canonical = self.state.canonical();
        self.rename(canonical)
    }

    pub(crate) fn rename(self, canonical: crate::state::Canonical) -> Self {
        let resource = self
            .flow
            .resource
            .into_iter()
            .filter_map(|(place, basis)| {
                let place = match place {
                    Place::World(index, id) => {
                        Place::World(canonical.world[index]?, canonical.resource[&id])
                    }
                    Place::Held(index, id) => {
                        Place::Held(canonical.frame[index]?, canonical.resource[&id])
                    }
                };
                Some((place, basis))
            })
            .collect();
        let mut context = vec![BTreeSet::new(); canonical.state.world.len()];
        for (index, target) in canonical.world.iter().enumerate() {
            if let Some(target) = target {
                context[*target] = self.flow.context[index].clone();
            }
        }
        let mut frame = vec![None; canonical.state.frame.len()];
        for (index, target) in canonical.frame.iter().enumerate() {
            if let Some(target) = target {
                frame[*target] = self.flow.frame[index];
            }
        }
        Self {
            state: canonical.state,
            flow: Flow {
                resource,
                context,
                frame,
            },
        }
    }
}
