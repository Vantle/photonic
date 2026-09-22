use super::cursor::Cursor;
use super::layer::{Active, Layer};
use super::playback::Playback;
use super::prefix::Prefix;
use super::slot::Slot;
use super::space::Space;
use crate::factor::Budget;
use crate::index::Index;
use std::sync::Arc;
use std::task::Poll;

struct Replay {
    position: usize,
    active: Active,
    playback: Playback,
}

pub(super) struct Update<'a> {
    pub index: &'a Index,
    pub order: &'a [usize],
    pub changed: &'a [usize],
    pub depth: Option<usize>,
}

pub(super) struct Tree {
    source: Cursor,
    budget: Arc<Budget>,
    layer: Vec<Layer>,
    playback: Option<Replay>,
    restoration: Option<usize>,
    cached: usize,
    enabled: bool,
}

impl Tree {
    pub fn new(width: usize, depth: [usize; 2], budget: Arc<Budget>) -> Self {
        let mut layer = depth.into_iter().map(Layer::new).collect::<Vec<_>>();
        layer.sort_by_key(|layer| layer.depth);
        layer.dedup_by_key(|layer| layer.depth);
        Self {
            source: Cursor::new(width),
            budget,
            layer,
            playback: None,
            restoration: None,
            cached: 0,
            enabled: true,
        }
    }

    pub fn depth(&self) -> usize {
        self.layer.last().unwrap().depth
    }

    pub fn update(&mut self, update: Update<'_>) {
        let Update {
            index,
            order,
            changed,
            depth,
        } = update;
        self.reset(index);
        for layer in &mut self.layer {
            layer.update(
                index,
                order[layer.depth + 1..order.len() - 1]
                    .iter()
                    .any(|position| changed.contains(position)),
            );
        }
        if let Some(depth) = depth
            && self.layer.len() < 8
            && let Err(position) = self.layer.binary_search_by_key(&depth, |layer| layer.depth)
        {
            self.layer.insert(position, Layer::new(depth));
        }
        self.cached = self.layer.iter().map(|layer| layer.cached).sum();
    }

    fn prepare(&mut self, space: &Space, order: &[usize], index: &Index) {
        if !self.enabled {
            return;
        }
        for (position, layer) in self.layer.iter_mut().enumerate() {
            let Some(candidate) = self.source.boundary(layer.depth) else {
                continue;
            };
            let Some(member) = space.domain[order[layer.depth]].get(candidate) else {
                continue;
            };
            let previous = layer.cached;
            layer.prepare(
                space.dependency(member.site, self.source.binding(), order, index),
                &self.budget,
                4096 - self.cached,
            );
            self.cached = self.cached - previous + layer.cached;
            if let Some(active) = layer.take() {
                self.playback = Some(Replay {
                    position,
                    active,
                    playback: Playback::default(),
                });
                self.record(position);
            }
            return;
        }
    }

    fn record(&mut self, position: usize) {
        let depth = self.layer[position].depth;
        let trace = &self.playback.as_ref().unwrap().active.trace;
        for layer in &mut self.layer[..position] {
            let previous = layer.cached;
            layer.extend(
                trace,
                &self.source.binding()[layer.depth..depth],
                4096 - self.cached,
            );
            self.cached = self.cached - previous + layer.cached;
        }
    }

    fn emit(&mut self, result: &Poll<Option<Vec<Slot>>>) {
        for layer in &mut self.layer {
            let previous = layer.cached;
            layer.append(result, 4096 - self.cached);
            if self.source.boundary(layer.depth).is_some() {
                layer.seal();
            }
            self.cached = self.cached - previous + layer.cached;
        }
    }

    #[inline]
    fn replay(&mut self, order: &[usize]) -> Option<Poll<Option<Vec<Slot>>>> {
        let replay = self.playback.as_mut()?;
        replay
            .playback
            .step(&replay.active.trace, order, self.source.binding())
    }

    fn advance(
        &mut self,
        space: &mut Space,
        order: &[usize],
        index: &Index,
    ) -> Poll<Option<Vec<Slot>>> {
        if let Some(replay) = self.playback.take() {
            let layer = &mut self.layer[replay.position];
            let candidate = self.source.boundary(layer.depth).unwrap();
            self.source.seek(layer.depth, candidate + 1);
            layer.restore(replay.active);
        }
        if let Some(progress) = self.restoration.take() {
            for _ in 0..progress {
                let _ = self.source.step(space, order, index);
            }
        }
        self.prepare(space, order, index);
        if let Some(result) = self.replay(order) {
            return result;
        }
        let result = self.source.step(space, order, index);
        self.emit(&result);
        result
    }
}

impl Prefix for Tree {
    fn waiting(&self) -> usize {
        self.playback
            .as_ref()
            .map_or(0, |replay| replay.playback.waiting(&replay.active.trace))
    }

    fn skip(&mut self, maximum: usize) -> usize {
        self.playback.as_mut().map_or(0, |replay| {
            replay.playback.skip(&replay.active.trace, maximum)
        })
    }

    fn reset(&mut self, _: &Index) {
        if let Some(replay) = self.playback.take() {
            self.layer[replay.position].restore(replay.active);
        }
        for layer in &mut self.layer {
            layer.finish();
        }
        self.cached = self.layer.iter().map(|layer| layer.cached).sum();
        self.source.reset();
        self.playback = None;
        self.restoration = None;
    }

    #[inline]
    fn step(
        &mut self,
        space: &mut Space,
        order: &[usize],
        index: &Index,
    ) -> Poll<Option<Vec<Slot>>> {
        if let Some(result) = self.replay(order) {
            return result;
        }
        self.advance(space, order, index)
    }

    fn evict(&mut self) {
        if let Some(replay) = &self.playback {
            self.restoration = Some(replay.playback.progress);
        }
        for layer in &mut self.layer {
            layer.clear();
        }
        self.playback = None;
        self.cached = 0;
        self.enabled = false;
    }

    fn cached(&self) -> usize {
        self.cached
    }

    fn retained(&self) -> usize {
        self.source.retained() + self.cached + self.layer.len() + 1
    }

    #[cfg(test)]
    fn size(&self) -> usize {
        assert_eq!(
            self.cached,
            self.layer.iter().map(|layer| layer.cached).sum::<usize>()
        );
        self.source.size()
            + self
                .layer
                .iter()
                .enumerate()
                .map(|(position, layer)| {
                    layer.size(
                        self.playback
                            .as_ref()
                            .filter(|replay| replay.position == position)
                            .map(|replay| &replay.active),
                    )
                })
                .sum::<usize>()
            + self.layer.len()
            + 1
    }
}
