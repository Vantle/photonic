use super::taxonomy::{Makeup, Taxonomy};
use crate::place::Place;

// A configuration built from its makeup holds the root frame first, then each kind in order: its
// frames after the root's, its worlds, and its token ids after the root's. The first world, frame
// and token id of every part, with the totals last, locate any place in its part.
#[derive(Debug)]
pub(super) struct Layout {
    world: Vec<usize>,
    frame: Vec<usize>,
    token: Vec<usize>,
}

pub(super) enum Site {
    Root,
    Part(usize),
}

// A part of one layout laid over another part of another layout: its worlds, frames and token ids
// keep their offsets from the part's start.
pub(super) struct Shift<'layout> {
    from: &'layout Layout,
    part: usize,
    to: &'layout Layout,
    other: usize,
}

impl Layout {
    pub fn new(taxonomy: &Taxonomy, makeup: &Makeup) -> Self {
        Self::of(taxonomy, makeup.root, &makeup.kind)
    }

    pub fn of(taxonomy: &Taxonomy, root: u32, kind: &[u32]) -> Self {
        let (mut world, mut frame, mut token) = (0, 1, taxonomy.token(root));
        let mut layout = Self {
            world: vec![world],
            frame: vec![frame],
            token: vec![token],
        };
        for &kind in kind {
            let size = taxonomy.kind(kind).1;
            world += size.world;
            frame += size.frame;
            token += size.token;
            layout.world.push(world);
            layout.frame.push(frame);
            layout.token.push(token);
        }
        layout
    }

    pub fn world(&self, world: usize) -> usize {
        self.world.partition_point(|&start| start <= world) - 1
    }

    pub fn frame(&self, frame: usize) -> Site {
        if frame == 0 {
            return Site::Root;
        }
        Site::Part(self.frame.partition_point(|&start| start <= frame) - 1)
    }

    pub fn site(&self, place: Place) -> Site {
        match place {
            Place::World(world, _) => Site::Part(self.world(world)),
            Place::Context(frame, _) | Place::Held(frame, _) => self.frame(frame),
        }
    }

    pub fn shift<'layout>(
        &'layout self,
        part: usize,
        to: &'layout Self,
        other: usize,
    ) -> Shift<'layout> {
        Shift {
            from: self,
            part,
            to,
            other,
        }
    }
}

impl Shift<'_> {
    pub fn world(&self, world: usize) -> usize {
        world - self.from.world[self.part] + self.to.world[self.other]
    }

    pub fn frame(&self, frame: usize) -> usize {
        frame - self.from.frame[self.part] + self.to.frame[self.other]
    }

    pub fn place(&self, place: Place) -> Place {
        let id = |id: usize| id - self.from.token[self.part] + self.to.token[self.other];
        match place {
            Place::World(world, token) => Place::World(self.world(world), id(token)),
            Place::Context(frame, token) => Place::Context(self.frame(frame), id(token)),
            Place::Held(frame, token) => Place::Held(self.frame(frame), id(token)),
        }
    }
}
