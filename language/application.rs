use crate::basis::Set;
use crate::flow::{Applied, Binding, Closure, Flow};
use crate::hashing::Builder;
use crate::place::Place;
use crate::program::Instruction;
use crate::state::{Frame, State, Token};
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub(crate) struct Request<'source> {
    pub source: &'source State,
    pub scope: &'source [crate::program::Scope],
    pub frame: usize,
    pub owner: Option<usize>,
    pub rule: &'source Instruction,
    pub binding: &'source Binding,
    pub closure: Option<Closure<'source>>,
}

struct Draft {
    resource: Vec<(Place, Set<Place>)>,
    context: Vec<Set<usize>>,
    frame: Vec<Option<usize>>,
}

struct Import<'state> {
    closure: Closure<'state>,
    frame: HashMap<usize, usize, Builder>,
    resource: HashMap<usize, usize, Builder>,
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
        state.frame.push(
            Frame {
                scope: original.scope,
                parent: None,
                lexical: None,
                particle: Default::default(),
                held: Vec::new(),
            }
            .into(),
        );
        flow.frame.push(None);
        let parent = original
            .parent
            .map(|index| self.include(index, state, flow));
        let lexical = original
            .lexical
            .map(|index| self.include(index, state, flow));
        let particle = self.particle(
            &original.particle,
            index,
            position,
            Place::Context,
            state,
            flow,
        );
        let held = self.particle(&original.held, index, position, Place::Held, state, flow);
        state.frame[position] = Frame {
            scope: original.scope,
            parent,
            lexical,
            particle: particle.into(),
            held,
        }
        .into();
        position
    }

    fn particle<'token>(
        &mut self,
        value: impl IntoIterator<Item = &'token Token>,
        source: usize,
        target: usize,
        place: impl Fn(usize, usize) -> Place,
        state: &mut State,
        flow: &mut Draft,
    ) -> Vec<Token> {
        let mut particle = BTreeMap::<usize, (Token, BTreeSet<Place>)>::new();
        for token in value {
            let id = *self.resource.entry(token.id).or_insert_with(|| {
                let next = self.next;
                self.next += 1;
                next
            });
            let capture = token.capture.map(|index| self.include(index, state, flow));
            let entry = particle.entry(id).or_insert_with(|| {
                (
                    Token {
                        id,
                        value: token.value,
                        capture,
                    },
                    BTreeSet::new(),
                )
            });
            entry.1.extend(
                self.closure.flow.resource[&place(source, token.id)]
                    .iter()
                    .copied(),
            );
        }
        particle
            .into_iter()
            .map(|(id, (token, basis))| {
                flow.resource.push((place(target, id), basis.into()));
                token
            })
            .collect()
    }
}

pub(crate) fn apply(request: Request<'_>) -> Applied {
    #[cfg(feature = "measurement")]
    let _measurement =
        crate::measurement::profile::Scope::new(crate::measurement::profile::Phase::Application);
    let Request {
        source,
        scope: catalog,
        frame,
        owner,
        rule,
        binding,
        closure,
    } = request;
    let mut state = State {
        world: source.world.clone(),
        frame: source.frame.clone(),
    };
    let mut flow = Draft {
        resource: state
            .frame
            .iter()
            .enumerate()
            .flat_map(|(index, frame)| {
                frame
                    .particle
                    .iter()
                    .map(move |token| Place::Context(index, token.id))
                    .chain(
                        frame
                            .held
                            .iter()
                            .map(move |token| Place::Held(index, token.id)),
                    )
                    .map(|place| (place, Set::single(place)))
            })
            .collect(),
        context: Vec::new(),
        frame: (0..source.frame.len()).map(Some).collect(),
    };
    let layout = crate::layout::Layout::new(source);
    let mut next = layout.resource;
    let owner = if let Some(closure) = closure {
        let mut resource = HashMap::default();
        for (index, frame) in closure.state.frame.iter().enumerate() {
            for (place, token) in frame
                .particle
                .iter()
                .map(|token| (Place::Context(index, token.id), token))
                .chain(
                    frame
                        .held
                        .iter()
                        .map(|token| (Place::Held(index, token.id), token)),
                )
            {
                let basis = &closure.flow.resource[&place];
                if basis.len() != 1
                    || token
                        .capture
                        .is_some_and(|frame| closure.flow.frame[frame].is_none())
                {
                    continue;
                }
                let original = source.token(*basis.first().unwrap());
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
            frame: HashMap::default(),
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
    let result = crate::evaluation::apply(crate::evaluation::Request {
        source,
        scope: catalog,
        frame,
        owner,
        rule,
        binding,
        state,
        next,
        layout: &layout,
    });
    let state = result.state;
    flow.frame.resize(state.frame.len(), None);
    let mut target = 0;
    for (index, world) in source.world.iter().enumerate() {
        if binding.world.contains(&index) {
            continue;
        }
        flow.context.push(Set::single(index));
        for token in &world.particle {
            flow.resource.push((
                Place::World(target, token.id),
                Set::single(Place::World(index, token.id)),
            ));
        }
        target += 1;
    }
    let consumed = if returning {
        source.frame[frame]
            .held
            .iter()
            .map(|token| Place::Held(frame, token.id))
            .collect::<Set<_>>()
    } else {
        Set::default()
    };
    let basis = binding
        .footprint
        .union(&consumed)
        .copied()
        .collect::<Set<_>>();
    let mut reserve = BTreeMap::<usize, BTreeSet<Place>>::new();
    if rule.output.iter().any(|output| output.body.is_some()) {
        for &place in binding.exact.union(&consumed) {
            let token = source.token(place).unwrap();
            reserve.entry(token.id).or_default().insert(place);
        }
    }
    for output in &rule.output {
        let selected = if output.body.is_some() {
            &binding.exact
        } else {
            &binding.footprint
        };
        let mut remainder = BTreeMap::<usize, BTreeSet<Place>>::new();
        for &index in &binding.world {
            for token in &source.world[index].particle {
                let place = Place::World(index, token.id);
                if !selected.contains(&place) {
                    remainder.entry(token.id).or_default().insert(place);
                }
            }
        }
        let world = &state.world[target];
        if output.body.is_some() {
            let frame = world.frame;
            flow.frame[frame] = None;
            for token in &state.frame[frame].held {
                flow.resource.push((
                    Place::Held(frame, token.id),
                    reserve[&token.id].iter().copied().collect(),
                ));
            }
            for token in &state.frame[frame].particle {
                flow.resource
                    .push((Place::Context(frame, token.id), basis.clone()));
            }
        }
        for token in &world.particle {
            let origin = if token.id >= next {
                basis.clone()
            } else {
                remainder[&token.id].iter().copied().collect()
            };
            flow.resource.push((Place::World(target, token.id), origin));
        }
        flow.context.push(binding.world.clone());
        target += 1;
    }
    flow.resource
        .retain(|(place, _)| state.token(*place).is_some());
    Applied {
        state,
        flow: Flow {
            resource: flow.resource.into_iter().collect(),
            context: flow.context,
            frame: flow.frame,
        },
    }
}
