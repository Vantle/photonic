use super::component::{decompose, extract};
use crate::basis::Set;
use crate::flow::Flow;
use crate::link::Link;
use crate::place::Place;
use crate::program::Symbol;
use crate::state::{Canonical, Frame, State, Token, World};
use hashing::Builder;
use indexmap::IndexMap;
use smallvec::SmallVec;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// Kinds are the canonical forms of components and roots the canonical forms of root frames, or of
// whole configurations whose root is tied to a component. Both are numbered in the order a run
// meets them, so a configuration is named by its root and its sorted kinds, and the numbering is
// the same for any number of workers because new forms are numbered in a fixed order. Naming an
// extracted component is remembered by its contents; the name is a function of them alone.
const SHARD: usize = 64;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(super) enum Root {
    Hub(Frame),
    Whole(State),
}

#[derive(Clone, Copy)]
pub(super) struct Size {
    pub world: usize,
    pub frame: usize,
    pub token: usize,
    pub occurrence: usize,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(super) struct Makeup {
    pub root: u32,
    pub kind: Vec<u32>,
}

pub(super) struct Piece {
    world: Vec<usize>,
    frame: Vec<usize>,
    normal: Vec<(usize, usize)>,
    named: Arc<Canonical>,
}

pub(super) enum Draft {
    Whole(Canonical),
    Split {
        root: Frame,
        rename: Vec<(usize, usize)>,
        piece: Vec<Piece>,
    },
}

pub(super) struct Renaming {
    pub world: Vec<Option<usize>>,
    pub frame: Vec<Option<usize>>,
    pub resource: crate::relation::Map<usize, usize>,
}

impl Renaming {
    pub fn place(&self, place: Place) -> Option<Place> {
        Some(match place {
            Place::World(index, id) => Place::World(self.world[index]?, self.resource[&id]),
            Place::Context(index, id) => Place::Context(self.frame[index]?, self.resource[&id]),
            Place::Held(index, id) => Place::Held(self.frame[index]?, self.resource[&id]),
        })
    }

    pub fn flow(&self, flow: Flow, world: usize, frame: usize) -> Flow {
        let resource = flow
            .resource
            .into_iter()
            .filter_map(|(place, basis)| Some((self.place(place)?, basis)))
            .collect();
        let mut context = vec![Set::default(); world];
        for (index, target) in self.world.iter().enumerate() {
            if let Some(target) = target {
                context[*target] = flow.context[index].clone();
            }
        }
        let mut mapping = vec![None; frame];
        for (index, target) in self.frame.iter().enumerate() {
            if let Some(target) = target {
                mapping[*target] = flow.frame[index];
            }
        }
        Flow {
            resource,
            context,
            frame: mapping,
        }
    }
}

pub(super) struct Taxonomy {
    kind: IndexMap<State, Size, Builder>,
    root: IndexMap<Root, usize, Builder>,
    name: Vec<Mutex<HashMap<State, Arc<Canonical>, Builder>>>,
}

impl Default for Taxonomy {
    fn default() -> Self {
        Self {
            kind: IndexMap::default(),
            root: IndexMap::default(),
            name: (0..SHARD).map(|_| Mutex::default()).collect(),
        }
    }
}

fn size(state: &State) -> Size {
    let mut token = state
        .world
        .iter()
        .flat_map(|world| world.particle.iter())
        .chain(state.frame.iter().flat_map(|frame| frame.token()))
        .map(|token| token.id)
        .collect::<Vec<_>>();
    token.sort_unstable();
    token.dedup();
    Size {
        world: state.world.len(),
        frame: state.frame.len() - 1,
        token: token.len(),
        occurrence: state.size(),
    }
}

fn hub(state: &State) -> (Frame, Vec<(usize, usize)>) {
    let frame = &state.frame[0];
    let mut incidence =
        HashMap::<usize, (Symbol, Option<usize>, SmallVec<[Link; 2]>), Builder>::default();
    for token in &frame.held {
        incidence
            .entry(token.id)
            .or_insert_with(|| (token.value, token.capture, SmallVec::new()))
            .2
            .push(Link::Holder);
    }
    for token in &frame.particle {
        incidence
            .entry(token.id)
            .or_insert_with(|| (token.value, token.capture, SmallVec::new()))
            .2
            .push(Link::Owner);
    }
    let mut order = incidence.into_iter().collect::<Vec<_>>();
    order.sort_by(|left, right| left.1.cmp(&right.1).then_with(|| left.0.cmp(&right.0)));
    let rename = order
        .iter()
        .enumerate()
        .map(|(position, (id, _))| (*id, position))
        .collect::<HashMap<_, _, Builder>>();
    let token = |token: &Token| Token {
        id: rename[&token.id],
        value: token.value,
        capture: token.capture,
    };
    let mut particle = frame.particle.iter().map(token).collect::<Vec<_>>();
    particle.sort_by_key(|token| token.id);
    let mut held = frame.held.iter().map(token).collect::<Vec<_>>();
    held.sort_by_key(|token| token.id);
    let named = Frame {
        scope: frame.scope,
        parent: frame.parent,
        lexical: frame.lexical,
        particle: particle.into(),
        held,
    };
    (named, rename.into_iter().collect())
}

impl Taxonomy {
    fn name(&self, state: State) -> Arc<Canonical> {
        let shard = &self.name[(hashing::value(&state) >> 40) as usize % SHARD];
        if let Some(found) = shard.lock().expect("an unpoisoned cache").get(&state) {
            return found.clone();
        }
        let named = Arc::new(state.canonical());
        shard
            .lock()
            .expect("an unpoisoned cache")
            .entry(state)
            .or_insert(named)
            .clone()
    }

    pub fn analyze(&self, state: &State) -> Draft {
        let Some(component) = decompose(state) else {
            return Draft::Whole(state.canonical());
        };
        let (root, rename) = hub(state);
        let piece = component
            .into_iter()
            .map(|value| {
                let (extracted, normal) = extract(state, &value);
                Piece {
                    world: value.world,
                    frame: value.frame,
                    normal,
                    named: self.name(extracted),
                }
            })
            .collect();
        Draft::Split {
            root,
            rename,
            piece,
        }
    }

    pub fn find(&self, state: &State) -> Option<Makeup> {
        let number = |id: usize| u32::try_from(id).ok();
        match self.analyze(state) {
            Draft::Whole(canonical) => Some(Makeup {
                root: number(self.root.get_index_of(&Root::Whole(canonical.state))?)?,
                kind: Vec::new(),
            }),
            Draft::Split { root, piece, .. } => {
                let mut kind = piece
                    .iter()
                    .map(|piece| number(self.kind.get_index_of(&piece.named.state)?))
                    .collect::<Option<Vec<_>>>()?;
                kind.sort_unstable();
                Some(Makeup {
                    root: number(self.root.get_index_of(&Root::Hub(root))?)?,
                    kind,
                })
            }
        }
    }

    pub fn kind(&self, id: u32) -> (&State, Size) {
        let (state, &size) = self
            .kind
            .get_index(id as usize)
            .expect("a kind is numbered before it is used");
        (state, size)
    }

    pub fn root(&self, id: u32) -> &Root {
        self.root
            .get_index(id as usize)
            .expect("a root is numbered before it is used")
            .0
    }

    pub fn token(&self, id: u32) -> usize {
        *self
            .root
            .get_index(id as usize)
            .expect("a root is numbered before it is used")
            .1
    }

    pub fn intern(&mut self, draft: &Draft) -> (u32, Vec<u32>) {
        match draft {
            Draft::Whole(canonical) => {
                let root = Root::Whole(canonical.state.clone());
                let id = self.root.insert_full(root, 0).0;
                (
                    u32::try_from(id).expect("fewer than 2^32 roots"),
                    Vec::new(),
                )
            }
            Draft::Split {
                root,
                rename,
                piece,
            } => {
                let id = match self.root.get_index_of(&Root::Hub(root.clone())) {
                    Some(id) => id,
                    None => {
                        self.root
                            .insert_full(Root::Hub(root.clone()), rename.len())
                            .0
                    }
                };
                let kind = piece
                    .iter()
                    .map(|piece| {
                        let state = &piece.named.state;
                        let id = match self.kind.get_index_of(state) {
                            Some(id) => id,
                            None => self.kind.insert_full(state.clone(), size(state)).0,
                        };
                        u32::try_from(id).expect("fewer than 2^32 kinds")
                    })
                    .collect();
                (u32::try_from(id).expect("fewer than 2^32 roots"), kind)
            }
        }
    }

    pub fn extent(&self, makeup: &Makeup) -> (usize, usize) {
        if let Root::Whole(state) = self.root(makeup.root) {
            return (state.world.len(), state.frame.len());
        }
        makeup.kind.iter().fold((0, 1), |(world, frame), &kind| {
            let size = self.kind(kind).1;
            (world + size.world, frame + size.frame)
        })
    }

    pub fn hub(&self, makeup: &Makeup) -> bool {
        matches!(self.root(makeup.root), Root::Hub(_))
    }

    pub fn measure(&self, makeup: &Makeup) -> (usize, usize, usize) {
        match self.root(makeup.root) {
            Root::Whole(state) => (state.world.len(), state.size(), state.reachable().len()),
            Root::Hub(frame) => makeup.kind.iter().fold(
                (0, frame.size(), 1),
                |(world, occurrence, scope), &kind| {
                    let size = self.kind(kind).1;
                    (
                        world + size.world,
                        occurrence + size.occurrence,
                        scope + size.frame,
                    )
                },
            ),
        }
    }

    pub fn assemble(
        &self,
        draft: Draft,
        root: u32,
        kind: &[u32],
        original: (usize, usize),
    ) -> (Makeup, Renaming) {
        let Draft::Split { rename, piece, .. } = draft else {
            let Draft::Whole(canonical) = draft else {
                unreachable!("a draft is whole or split")
            };
            let makeup = Makeup {
                root,
                kind: Vec::new(),
            };
            let renaming = Renaming {
                world: canonical.world,
                frame: canonical.frame,
                resource: canonical.resource,
            };
            return (makeup, renaming);
        };
        let mut order = (0..piece.len()).collect::<Vec<_>>();
        order.sort_by_key(|&index| kind[index]);
        let mut world = vec![None; original.0];
        let mut frame = vec![None; original.1];
        frame[0] = Some(0);
        let mut resource = rename;
        let mut offset = (0, 1, self.token(root));
        for &index in &order {
            let piece = &piece[index];
            let size = self.kind(kind[index]).1;
            for (position, &original) in piece.world.iter().enumerate() {
                let named = piece.named.world[position].expect("a named kind keeps its worlds");
                world[original] = Some(offset.0 + named);
            }
            for (position, &original) in piece.frame.iter().enumerate() {
                let named = piece.named.frame[position + 1].expect("a named kind keeps its frames");
                frame[original] = Some(offset.1 + named - 1);
            }
            for &(original, normalized) in &piece.normal {
                resource.push((original, offset.2 + piece.named.resource[&normalized]));
            }
            offset = (
                offset.0 + size.world,
                offset.1 + size.frame,
                offset.2 + size.token,
            );
        }
        let makeup = Makeup {
            root,
            kind: order.iter().map(|&index| kind[index]).collect(),
        };
        let renaming = Renaming {
            world,
            frame,
            resource: resource.into_iter().collect(),
        };
        (makeup, renaming)
    }

    pub fn materialize(&self, makeup: &Makeup) -> State {
        let root = match self.root(makeup.root) {
            Root::Whole(state) => return state.clone(),
            Root::Hub(frame) => frame.clone(),
        };
        let mut world = Vec::new();
        let mut frame = vec![Arc::new(root)];
        let mut offset = (1, self.token(makeup.root));
        for &kind in &makeup.kind {
            let (state, size) = self.kind(kind);
            let base = offset.0 - 1;
            let shift = offset.1;
            let place = |index: usize| if index == 0 { 0 } else { base + index };
            let token = |token: &Token| Token {
                id: token.id + shift,
                value: token.value,
                capture: token.capture.map(place),
            };
            for value in state.frame.iter().skip(1) {
                let particle = value.particle.iter().map(token).collect::<Vec<_>>();
                frame.push(Arc::new(Frame {
                    scope: value.scope,
                    parent: value.parent.map(place),
                    lexical: value.lexical.map(place),
                    particle: particle.into(),
                    held: value.held.iter().map(token).collect(),
                }));
            }
            for value in &state.world {
                world.push(Arc::new(World {
                    frame: place(value.frame),
                    particle: value.particle.iter().map(token).collect(),
                }));
            }
            offset = (offset.0 + size.frame, offset.1 + size.token);
        }
        State {
            world: world.into_iter().collect(),
            frame: frame.into_iter().collect(),
        }
    }
}
