use super::component::{decompose, extract};
use super::makeup::Makeup;
use super::memo::Memo;
use super::size::Size;
use crate::canonical::Exhausted;
use crate::executor::Executor;
use crate::link::Link;
use crate::program::Symbol;
use crate::runtime::Measure;
use crate::state::{Canonical, Frame, Renaming, State, Token, World};
use hashing::Builder;
use indexmap::IndexMap;
use smallvec::SmallVec;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(super) enum Root {
    Hub(Frame),
    Whole(State),
}

// How many coherences and frames a configuration holds.
#[derive(Clone, Copy)]
pub(super) struct Extent {
    pub world: usize,
    pub frame: usize,
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
        original: Extent,
    },
}

// Kinds are the canonical forms of components and roots the canonical forms of root frames, or of
// whole configurations whose root is tied to a component. Both are numbered in the order a run
// meets them, so a configuration is named by its root and its sorted kinds, and the numbering is
// the same for any number of workers because new forms are numbered in a fixed order. Naming an
// extracted component is remembered by its contents; the name is a function of them alone.
#[derive(Default)]
pub(super) struct Taxonomy {
    kind: IndexMap<State, Size, Builder>,
    root: IndexMap<Root, usize, Builder>,
    name: Memo<State, Name>,
}

// What naming a configuration found: its canonical form with the steps its search charged, or that
// the search ran out of the allowance it had. A remembered name charges the same steps each time it
// serves, so what a run charges does not depend on which worker named it first.
#[derive(Clone)]
pub(super) enum Name {
    Found(Arc<Canonical>, usize),
    Exhausted(usize),
}

impl Name {
    pub fn search(
        allowance: usize,
        search: impl FnOnce(&mut usize) -> Result<Canonical, Exhausted>,
    ) -> Self {
        let mut left = allowance;
        match search(&mut left) {
            Ok(named) => Self::Found(Arc::new(named), allowance - left),
            Err(Exhausted) => Self::Exhausted(allowance),
        }
    }

    // The name, charging its steps to the budget, or none, with the budget spent, when it takes
    // more than the budget holds.
    pub fn spend(self, budget: &mut usize) -> Result<Arc<Canonical>, Exhausted> {
        match self {
            Self::Found(named, cost) if cost <= *budget => {
                *budget -= cost;
                Ok(named)
            }
            Self::Found(..) | Self::Exhausted(_) => {
                *budget = 0;
                Err(Exhausted)
            }
        }
    }
}

// Roots and kinds are numbered in 32 bits.
fn number(id: usize) -> u32 {
    u32::try_from(id).expect("fewer than 2^32 roots and kinds")
}

fn hub(state: &State) -> (Frame, Vec<(usize, usize)>) {
    let frame = &state.frame[0];
    let mut incidence =
        HashMap::<usize, (Symbol, Option<usize>, SmallVec<[Link; 2]>), Builder>::default();
    let held = frame.held.iter().map(|token| (token, Link::Holder));
    let particle = frame.particle.iter().map(|token| (token, Link::Owner));
    for (token, link) in held.chain(particle) {
        incidence
            .entry(token.id)
            .or_insert_with(|| (token.value, token.capture, SmallVec::new()))
            .2
            .push(link);
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
    // Names are remembered for one batch of applications, where the components they leave
    // untouched repeat; remembering every name would keep a copy of every component ever made.
    pub fn forget(&mut self, executor: Option<&Executor>) {
        self.name.forget(executor);
    }

    // A remembered failure serves only budgets no larger than the one it failed with, and a larger
    // one searches again, so whether a name fits a budget never depends on which worker searched
    // for it first.
    fn name(&self, state: State, budget: &mut usize) -> Result<Arc<Canonical>, Exhausted> {
        let allowance = *budget;
        let found = match self.name.find(&state) {
            Some(Name::Exhausted(tried)) if tried < allowance => {
                Name::search(allowance, |left| state.canonical(left))
            }
            Some(found) => found,
            None => {
                let made = Name::search(allowance, |left| state.canonical(left));
                self.name.get(state, |_| made.clone());
                made
            }
        };
        found.spend(budget)
    }

    // A configuration named by its parts, or whole when its root ties to one of them, spending the
    // budget on every search past the free steps; none when the budget runs out.
    pub fn analyze(&self, state: &State, budget: &mut usize) -> Result<Draft, Exhausted> {
        let Some(component) = decompose(state) else {
            return Ok(Draft::Whole(state.canonical(budget)?));
        };
        let (root, rename) = hub(state);
        let piece = component
            .into_iter()
            .map(|value| {
                let (extracted, normal) = extract(state, &value);
                Ok(Piece {
                    world: value.world,
                    frame: value.frame,
                    normal,
                    named: self.name(extracted, budget)?,
                })
            })
            .collect::<Result<_, Exhausted>>()?;
        Ok(Draft::Split {
            root,
            rename,
            piece,
            original: Extent {
                world: state.world.len(),
                frame: state.frame.len(),
            },
        })
    }

    pub fn find(&self, state: &State, budget: &mut usize) -> Result<Option<Makeup>, Exhausted> {
        Ok(match self.analyze(state, budget)? {
            Draft::Whole(canonical) => {
                self.root
                    .get_index_of(&Root::Whole(canonical.state))
                    .map(|root| Makeup {
                        root: number(root),
                        kind: Vec::new(),
                    })
            }
            Draft::Split { root, piece, .. } => {
                let kind = piece
                    .iter()
                    .map(|piece| self.kind.get_index_of(&piece.named.state).map(number))
                    .collect::<Option<Vec<_>>>();
                let root = self.root.get_index_of(&Root::Hub(root));
                kind.zip(root).map(|(mut kind, root)| {
                    kind.sort_unstable();
                    Makeup {
                        root: number(root),
                        kind,
                    }
                })
            }
        })
    }

    pub fn kind(&self, id: u32) -> &State {
        self.kind
            .get_index(id as usize)
            .expect("a kind is numbered before it is used")
            .0
    }

    pub fn size(&self, id: u32) -> Size {
        *self
            .kind
            .get_index(id as usize)
            .expect("a kind is numbered before it is used")
            .1
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
                (number(self.root.insert_full(root, 0).0), Vec::new())
            }
            Draft::Split {
                root,
                rename,
                piece,
                ..
            } => {
                let id = self
                    .root
                    .insert_full(Root::Hub(root.clone()), rename.len())
                    .0;
                let kind = piece
                    .iter()
                    .map(|piece| {
                        let state = &piece.named.state;
                        let id = match self.kind.get_index_of(state) {
                            Some(id) => id,
                            None => self.kind.insert_full(state.clone(), Size::new(state)).0,
                        };
                        number(id)
                    })
                    .collect();
                (number(id), kind)
            }
        }
    }

    pub fn extent(&self, makeup: &Makeup) -> Extent {
        if let Root::Whole(state) = self.root(makeup.root) {
            return Extent {
                world: state.world.len(),
                frame: state.frame.len(),
            };
        }
        makeup
            .kind
            .iter()
            .fold(Extent { world: 0, frame: 1 }, |extent, &kind| {
                let size = self.size(kind);
                Extent {
                    world: extent.world + size.world,
                    frame: extent.frame + size.frame,
                }
            })
    }

    // Whether a configuration is named by its parts, not whole.
    pub fn split(&self, makeup: &Makeup) -> bool {
        matches!(self.root(makeup.root), Root::Hub(_))
    }

    pub fn measure(&self, makeup: &Makeup) -> Measure {
        let frame = match self.root(makeup.root) {
            Root::Whole(state) => return Measure::new(state),
            Root::Hub(frame) => frame,
        };
        let start = Measure {
            coherence: 0,
            occurrence: frame.held.len(),
            scope: 0,
        };
        makeup.kind.iter().fold(start, |measure, &kind| {
            let size = self.size(kind);
            Measure {
                coherence: measure.coherence + size.world,
                occurrence: measure.occurrence + size.occurrence,
                scope: measure.scope + size.frame,
            }
        })
    }

    pub fn assemble(&self, draft: Draft, root: u32, kind: &[u32]) -> (Makeup, Renaming) {
        let (rename, piece, original) = match draft {
            Draft::Whole(canonical) => {
                let makeup = Makeup {
                    root,
                    kind: Vec::new(),
                };
                return (makeup, canonical.renaming);
            }
            Draft::Split {
                rename,
                piece,
                original,
                ..
            } => (rename, piece, original),
        };
        let mut order = (0..piece.len()).collect::<Vec<_>>();
        order.sort_by_key(|&index| kind[index]);
        let mut world = vec![None; original.world];
        let mut frame = vec![None; original.frame];
        frame[0] = Some(0);
        let mut resource = rename;
        let mut start = Size {
            world: 0,
            frame: 1,
            token: self.token(root),
            occurrence: 0,
        };
        for &index in &order {
            let piece = &piece[index];
            let size = self.size(kind[index]);
            for (position, &original) in piece.world.iter().enumerate() {
                let named =
                    piece.named.renaming.world[position].expect("a named kind keeps its worlds");
                world[original] = Some(start.world + named);
            }
            for (position, &original) in piece.frame.iter().enumerate() {
                let named = piece.named.renaming.frame[position + 1]
                    .expect("a named kind keeps its frames");
                frame[original] = Some(start.frame + named - 1);
            }
            for &(original, normalized) in &piece.normal {
                resource.push((
                    original,
                    start.token + piece.named.renaming.resource[&normalized],
                ));
            }
            start.world += size.world;
            start.frame += size.frame;
            start.token += size.token;
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
        let mut base = 0;
        let mut shift = self.token(makeup.root);
        for &kind in &makeup.kind {
            let (state, size) = (self.kind(kind), self.size(kind));
            let lift = |index: usize| if index == 0 { 0 } else { base + index };
            let token = |token: &Token| Token {
                id: token.id + shift,
                value: token.value,
                capture: token.capture.map(lift),
            };
            for value in state.frame.iter().skip(1) {
                let particle = value.particle.iter().map(token).collect::<Vec<_>>();
                frame.push(Arc::new(Frame {
                    scope: value.scope,
                    parent: value.parent.map(lift),
                    lexical: value.lexical.map(lift),
                    particle: particle.into(),
                    held: value.held.iter().map(token).collect(),
                }));
            }
            for value in &state.world {
                world.push(Arc::new(World {
                    frame: lift(value.frame),
                    particle: value.particle.iter().map(token).collect(),
                }));
            }
            base += size.frame;
            shift += size.token;
        }
        State {
            world: world.into_iter().collect(),
            frame: frame.into_iter().collect(),
        }
    }
}
