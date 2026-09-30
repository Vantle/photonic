use crate::program::Symbol;
use crate::refinement::Refinement;
use crate::state::{Frame, State, Token, World};
use hashing::Builder;
use std::collections::HashMap;
use std::sync::Arc;

// A part of a configuration as a configuration of its own, with a stand-in for every anchor it
// touches: the root stays frame zero, another anchor frame becomes a frame of a scope no program
// has, kept reachable by an empty coherence of its own, an anchor coherence becomes a coherence of
// the root around a token no program has, and an anchor token becomes such a token. Each stand-in
// carries its anchor's color, so two parts extract alike exactly when one maps onto the other with
// every anchor they touch fixed. The part's own coherences and frames come last, in the order of
// these lists.
pub(super) struct Extraction {
    pub state: State,
    pub world: Vec<usize>,
    pub frame: Vec<usize>,
}

// Colors number the vertices of one configuration, so marks count down from the largest integer,
// past every scope and atom a program has.
fn mark(color: usize) -> usize {
    usize::MAX - color
}

enum Name {
    Own(usize),
    Anchor(Token),
}

struct Naming {
    frame: HashMap<usize, usize, Builder>,
    token: HashMap<usize, Name, Builder>,
}

impl Naming {
    fn own(&self, token: &Token) -> Option<Token> {
        match self.token.get(&token.id)? {
            Name::Own(id) => Some(Token {
                id: *id,
                value: token.value,
                capture: token.capture.map(|frame| self.frame[&frame]),
            }),
            Name::Anchor(_) => None,
        }
    }

    fn any(&self, token: &Token) -> Token {
        match &self.token[&token.id] {
            Name::Own(_) => self.own(token).expect("an own token has a name"),
            Name::Anchor(stand) => stand.clone(),
        }
    }

    // The part's tokens among these, where an anchor holds them.
    fn held<'token>(&self, token: impl IntoIterator<Item = &'token Token>) -> Vec<Token> {
        token
            .into_iter()
            .filter_map(|token| self.own(token))
            .collect()
    }

    // Every token of a place of the part, an anchor's as its stand-in.
    fn every<'token>(&self, token: impl IntoIterator<Item = &'token Token>) -> Vec<Token> {
        token.into_iter().map(|token| self.any(token)).collect()
    }

    fn link(&self, frame: Option<usize>) -> Option<usize> {
        frame.map(|frame| self.frame[&frame])
    }
}

fn marked(id: usize, color: usize) -> Token {
    Token {
        id,
        value: Symbol::Atom(mark(color)),
        capture: None,
    }
}

pub(super) fn extract(
    state: &State,
    refinement: &Refinement,
    anchor: &[bool],
    vertex: &[usize],
) -> Extraction {
    let incidence = &refinement.incidence;
    let color = &refinement.color;
    let count = state.world.len();
    let resource = count + incidence.retained.len();
    let mut touched = vertex
        .iter()
        .flat_map(|&index| incidence.edge[index].iter().map(|&(_, target)| target))
        .filter(|&target| anchor[target] && target != count)
        .collect::<Vec<_>>();
    touched.sort_unstable_by_key(|&target| color[target]);
    touched.dedup();
    let stand = touched
        .iter()
        .filter(|&&target| (count..resource).contains(&target))
        .map(|&target| (incidence.retained[target - count], color[target]))
        .collect::<Vec<_>>();
    let world = vertex
        .iter()
        .copied()
        .filter(|&index| index < count)
        .collect::<Vec<_>>();
    let frame = vertex
        .iter()
        .filter(|&&index| (count..resource).contains(&index))
        .map(|&index| incidence.retained[index - count])
        .collect::<Vec<_>>();
    let mut naming = Naming {
        frame: HashMap::default(),
        token: HashMap::default(),
    };
    naming.frame.insert(0, 0);
    for (position, &(index, _)) in stand.iter().enumerate() {
        naming.frame.insert(index, 1 + position);
    }
    for (position, &index) in frame.iter().enumerate() {
        naming.frame.insert(index, 1 + stand.len() + position);
    }
    for &index in vertex.iter().filter(|&&index| index >= resource) {
        let id = naming.token.len();
        naming
            .token
            .insert(incidence.resource[index - resource], Name::Own(id));
    }
    for &target in touched.iter().filter(|&&target| target >= resource) {
        let id = naming.token.len();
        naming.token.insert(
            incidence.resource[target - resource],
            Name::Anchor(marked(id, color[target])),
        );
    }
    let root = &state.frame[0];
    let mut result = State {
        world: Default::default(),
        frame: Default::default(),
    };
    result.frame.push(Arc::new(Frame {
        scope: root.scope,
        parent: None,
        lexical: None,
        particle: naming.held(&root.particle).into(),
        held: naming.held(&root.held),
    }));
    for (position, &(index, color)) in stand.iter().enumerate() {
        let value = &state.frame[index];
        result.frame.push(Arc::new(Frame {
            scope: mark(color),
            parent: None,
            lexical: None,
            particle: naming.held(&value.particle).into(),
            held: naming.held(&value.held),
        }));
        result.world.push(Arc::new(World {
            frame: 1 + position,
            particle: Vec::new(),
        }));
    }
    for &index in &frame {
        let value = &state.frame[index];
        result.frame.push(Arc::new(Frame {
            scope: value.scope,
            parent: naming.link(value.parent),
            lexical: naming.link(value.lexical),
            particle: naming.every(&value.particle).into(),
            held: naming.every(&value.held),
        }));
    }
    let marker = naming.token.len()..;
    for (id, &target) in marker.zip(touched.iter().filter(|&&target| target < count)) {
        let mut particle = vec![marked(id, color[target])];
        particle.extend(naming.held(&state.world[target].particle));
        result.world.push(Arc::new(World { frame: 0, particle }));
    }
    for &index in &world {
        let value = &state.world[index];
        result.world.push(Arc::new(World {
            frame: naming.frame[&value.frame],
            particle: naming.every(&value.particle),
        }));
    }
    Extraction {
        state: result,
        world,
        frame,
    }
}
