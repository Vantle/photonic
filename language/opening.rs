use crate::program::{Scope, Symbol};
use crate::state::{Frame, State, Token, World};
use smallvec::SmallVec;
use std::sync::Arc;

pub(crate) struct Opening<'source, Vacant> {
    pub scope: &'source [Scope],
    pub held: &'source [Token],
    pub base: &'source [&'source Token],
    pub capture: usize,
    pub vacant: Vacant,
    pub next: usize,
    pub opened: Vec<usize>,
}

impl<'source, Vacant: Iterator<Item = usize>> Opening<'source, Vacant> {
    pub(crate) fn apply(
        &mut self,
        state: &mut State,
        body: usize,
        parent: Option<usize>,
        lexical: Option<usize>,
    ) {
        let scope = self.scope;
        self.open(state, body, &scope[body], parent, lexical);
    }

    pub(crate) fn open(
        &mut self,
        state: &mut State,
        body: usize,
        value: &Scope,
        parent: Option<usize>,
        lexical: Option<usize>,
    ) {
        let table = self.scope;
        let mut pending = SmallVec::<[(usize, &Scope, Option<usize>, Option<usize>); 4]>::new();
        pending.push((body, value, parent, lexical));
        while let Some((body, scope, parent, lexical)) = pending.pop() {
            let target = self.vacant.next().unwrap_or(state.frame.len());
            let value = Frame {
                scope: body,
                parent,
                lexical,
                particle: scope
                    .rule
                    .iter()
                    .map(|&rule| Token::new(Symbol::Rule(rule), target, &mut self.next))
                    .collect(),
                held: self.held.to_vec(),
            }
            .into();
            if target == state.frame.len() {
                state.frame.push(value);
            } else {
                state.frame[target] = value;
            }
            self.opened.push(target);
            for particle in &scope.initial {
                state.world.push(world(
                    target,
                    self.base,
                    particle,
                    self.capture,
                    &mut self.next,
                ));
            }
            pending.extend(
                scope
                    .scope
                    .iter()
                    .rev()
                    .map(|&body| (body, &table[body], Some(target), Some(target))),
            );
        }
    }
}

pub(crate) fn world(
    frame: usize,
    base: &[&Token],
    particle: &[Symbol],
    capture: usize,
    next: &mut usize,
) -> Arc<World> {
    World {
        frame,
        particle: base
            .iter()
            .copied()
            .cloned()
            .chain(
                particle
                    .iter()
                    .map(|&value| Token::new(value, capture, next)),
            )
            .collect(),
    }
    .into()
}
