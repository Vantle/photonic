use super::{particle, symbol};
use crate::accumulator::Accumulator;
use crate::hashing::mix;
use crate::profile;
use crate::state::{Frame, State};
use std::sync::Arc;

#[derive(Clone)]
pub(super) struct Index {
    value: [Arc<Vec<u64>>; 3],
    dependency: Option<Arc<super::dependency::Index>>,
}

fn initial(frame: &Frame) -> u64 {
    let _scope = profile::Scope::new(profile::Phase::Color);
    mix(frame.scope as u64)
        .wrapping_add(Accumulator::collect(
            frame.held.iter().map(|token| symbol(token.value)),
        ))
        .wrapping_add(
            Accumulator::collect(frame.particle.iter().map(|token| symbol(token.value)))
                .rotate_left(7),
        )
}

fn refine(frame: &Frame, previous: &[u64]) -> u64 {
    let _scope = profile::Scope::new(profile::Phase::Color);
    mix(frame.scope as u64)
        .wrapping_add(particle(&frame.held, previous))
        .wrapping_add(particle(&frame.particle, previous).rotate_left(7))
        .wrapping_add(
            frame
                .parent
                .map_or(0, |index| previous[index])
                .rotate_left(13),
        )
        .wrapping_add(
            frame
                .lexical
                .map_or(0, |index| previous[index])
                .rotate_left(37),
        )
}

impl Index {
    pub fn new(state: &State) -> Self {
        let empty = Self {
            value: std::array::from_fn(|_| Arc::new(Vec::new())),
            dependency: Some(Arc::new(Default::default())),
        };
        empty.advance(state, &[]).0
    }

    pub fn advance(&self, state: &State, changed: &[usize]) -> (Self, Vec<usize>) {
        let mut changed = changed
            .iter()
            .copied()
            .chain(self.value[0].len()..state.frame.len())
            .chain(state.frame.len()..self.value[0].len())
            .collect::<Vec<_>>();
        changed.sort_unstable();
        changed.dedup();
        if changed.is_empty() {
            return (self.clone(), Vec::new());
        }
        let dependency = self
            .dependency
            .as_ref()
            .map(|dependency| Arc::new(dependency.advance(state, &changed)));
        if dependency.is_none() {
            changed = (0..state.frame.len()).collect();
        }
        let mut index = Self {
            value: self.value.clone(),
            dependency,
        };
        changed.retain(|&frame| frame < state.frame.len());
        let mut affected = changed.clone();
        for phase in 0..3 {
            let mut value = (*self.value[phase]).clone();
            value.resize(state.frame.len(), 0);
            let mut altered = Vec::new();
            for &frame in &affected {
                let hash = if phase == 0 {
                    initial(&state.frame[frame])
                } else {
                    refine(&state.frame[frame], &index.value[phase - 1])
                };
                if self.value[phase].get(frame).copied() != Some(hash) {
                    altered.push(frame);
                }
                value[frame] = hash;
            }
            index.value[phase] = Arc::new(value);
            if phase == 2 {
                return (index, altered);
            }
            affected = changed.clone();
            if let Some(dependency) = &index.dependency {
                affected.extend(
                    altered
                        .iter()
                        .flat_map(|&frame| dependency.dependent(frame).copied()),
                );
            }
            affected.sort_unstable();
            affected.dedup();
        }
        unreachable!()
    }

    pub fn color(&self) -> &[u64] {
        &self.value[2]
    }

    pub fn retained(&self) -> usize {
        self.value.iter().map(|value| value.len()).sum::<usize>()
            + self
                .dependency
                .as_ref()
                .map_or(0, |dependency| dependency.retained())
    }

    pub fn evict(&mut self) -> usize {
        self.dependency
            .take()
            .map_or(0, |dependency| dependency.retained())
    }
}
