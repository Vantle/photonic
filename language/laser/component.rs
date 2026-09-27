use super::forest::Forest;
use crate::state::{Frame, State, Token, World};
use hashing::Builder;
use std::collections::HashMap;
use std::sync::Arc;

// A component is worlds and frames other than the root joined by shared tokens, frame links and
// captures. The root is a hub, not a link: a component refers to the root only as frame zero. A
// root that shares a token with a component or refers to one of its frames is tied to it, and a
// configuration with a tied root is not split; the condition survives renaming, so equal
// configurations are always split the same way.
pub(super) struct Component {
    pub world: Vec<usize>,
    pub frame: Vec<usize>,
}

pub(super) fn decompose(state: &State) -> Option<Vec<Component>> {
    let reachable = state.reachable();
    let count = state.world.len();
    let mut node = vec![usize::MAX; state.frame.len()];
    for (position, &index) in reachable.iter().enumerate().skip(1) {
        node[index] = count + position - 1;
    }
    let root = &state.frame[0];
    if root.parent.is_some_and(|index| index != 0) || root.lexical.is_some_and(|index| index != 0) {
        return None;
    }
    let mut owner = HashMap::<usize, usize, Builder>::default();
    for token in root.particle.iter().chain(&root.held) {
        if token.capture.is_some_and(|capture| capture != 0) {
            return None;
        }
        owner.insert(token.id, usize::MAX);
    }
    let mut forest = Forest::new(count + reachable.len() - 1);
    let mut join = |forest: &mut Forest, holder: usize, token: &Token| -> bool {
        if let Some(capture) = token.capture.filter(|&capture| capture != 0) {
            forest.union(holder, node[capture]);
        }
        match owner.get(&token.id) {
            Some(&usize::MAX) => false,
            Some(&other) => {
                forest.union(holder, other);
                true
            }
            None => {
                owner.insert(token.id, holder);
                true
            }
        }
    };
    for (index, world) in state.world.iter().enumerate() {
        if world.frame != 0 {
            forest.union(index, node[world.frame]);
        }
        for token in &world.particle {
            if !join(&mut forest, index, token) {
                return None;
            }
        }
    }
    for &index in &reachable[1..] {
        let value = &state.frame[index];
        for link in value.parent.into_iter().chain(value.lexical) {
            if link != 0 {
                forest.union(node[index], node[link]);
            }
        }
        for token in value.particle.iter().chain(&value.held) {
            if !join(&mut forest, node[index], token) {
                return None;
            }
        }
    }
    let mut group = HashMap::<usize, usize, Builder>::default();
    let mut component = Vec::<Component>::new();
    let mut place = |root: usize, component: &mut Vec<Component>| {
        *group.entry(root).or_insert_with(|| {
            component.push(Component {
                world: Vec::new(),
                frame: Vec::new(),
            });
            component.len() - 1
        })
    };
    for index in 0..count {
        let position = place(forest.find(index), &mut component);
        component[position].world.push(index);
    }
    for &index in &reachable[1..] {
        let position = place(forest.find(node[index]), &mut component);
        component[position].frame.push(index);
    }
    Some(component)
}

// A component is extracted with a stand-in root at frame zero and its token ids numbered in the
// order they appear, so the same component read from any configuration gives the same state.
pub(super) fn extract(state: &State, component: &Component) -> (State, Vec<(usize, usize)>) {
    let mut local = vec![usize::MAX; state.frame.len()];
    local[0] = 0;
    for (position, &index) in component.frame.iter().enumerate() {
        local[index] = position + 1;
    }
    let mut normal = HashMap::<usize, usize, Builder>::default();
    let mut rename = |token: &Token| {
        let next = normal.len();
        Token {
            id: *normal.entry(token.id).or_insert(next),
            value: token.value,
            capture: token.capture.map(|capture| local[capture]),
        }
    };
    let mut world = Vec::with_capacity(component.world.len());
    for &index in &component.world {
        let value = &state.world[index];
        world.push(Arc::new(World {
            frame: local[value.frame],
            particle: value.particle.iter().map(&mut rename).collect(),
        }));
    }
    let mut frame = Vec::with_capacity(component.frame.len() + 1);
    frame.push(Arc::new(Frame {
        scope: 0,
        parent: None,
        lexical: None,
        particle: Vec::new().into(),
        held: Vec::new(),
    }));
    for &index in &component.frame {
        let value = &state.frame[index];
        let particle = value.particle.iter().map(&mut rename).collect::<Vec<_>>();
        let held = value.held.iter().map(&mut rename).collect();
        frame.push(Arc::new(Frame {
            scope: value.scope,
            parent: value.parent.map(|parent| local[parent]),
            lexical: value.lexical.map(|lexical| local[lexical]),
            particle: particle.into(),
            held,
        }));
    }
    let state = State {
        world: world.into_iter().collect(),
        frame: frame.into_iter().collect(),
    };
    (state, normal.into_iter().collect())
}
