use super::Search;
use super::extraction::{Extraction, extract};
use crate::refinement::Refinement;
use crate::state::{Canonical, State};
use std::sync::Arc;

// A configuration named by its parts. The anchors are the coherences, frames and tokens the
// refinement tells apart from everything else, and they keep the places their colors give them.
// What remains falls into parts, joined by incidences that avoid the anchors, so no part touches
// another. Each part is named on its own, with a stand-in for each anchor it touches, and the parts
// follow the anchors in the order of their names: interchangeable parts cost a name each and a
// sort instead of every ordering of them.
pub(super) struct Division {
    state: Arc<State>,
    world: Vec<usize>,
    frame: Vec<usize>,
    part: Vec<Part>,
    current: usize,
}

struct Part {
    search: Search,
    world: Placement,
    frame: Placement,
}

// Where a part's own coherences or frames lie in its extracted configuration: from the start on,
// in the order of these originals.
struct Placement {
    start: usize,
    original: Vec<usize>,
}

impl Placement {
    // The originals in the order the part's name gives them.
    fn order(&self, position: &[Option<usize>]) -> Vec<usize> {
        let mut placed = self
            .original
            .iter()
            .enumerate()
            .map(|(offset, &index)| {
                let named = position[self.start + offset]
                    .expect("a part's name keeps its coherences and frames");
                (named, index)
            })
            .collect::<Vec<_>>();
        placed.sort_unstable();
        placed.into_iter().map(|(_, index)| index).collect()
    }
}

// The coherences and frames each part holds, found from every coherence and frame that is no
// anchor across the incidences between vertices that are none.
fn gather(refinement: &Refinement, anchor: &[bool], count: usize) -> Vec<Vec<usize>> {
    let edge = &refinement.incidence.edge;
    let mut seen = anchor.to_vec();
    let mut part = Vec::new();
    for start in 0..count {
        if std::mem::replace(&mut seen[start], true) {
            continue;
        }
        let mut vertex = vec![start];
        let mut cursor = 0;
        while let Some(&index) = vertex.get(cursor) {
            cursor += 1;
            for &(_, target) in &edge[index] {
                if !std::mem::replace(&mut seen[target], true) {
                    vertex.push(target);
                }
            }
        }
        vertex.sort_unstable();
        part.push(vertex);
    }
    part
}

impl Division {
    // A division into two or more parts, or none when the anchors leave at most one.
    pub fn new(state: Arc<State>, refinement: &Refinement) -> Option<Self> {
        let color = &refinement.color;
        let retained = &refinement.incidence.retained;
        let mut size = vec![0usize; color.len()];
        for &value in color {
            size[value] += 1;
        }
        let anchor = color
            .iter()
            .map(|&value| size[value] == 1)
            .collect::<Vec<_>>();
        let count = state.world.len();
        let resource = count + retained.len();
        let part = gather(refinement, &anchor, resource);
        if part.len() < 2 {
            return None;
        }
        let mut world = (0..count)
            .filter(|&index| anchor[index])
            .collect::<Vec<_>>();
        world.sort_unstable_by_key(|&index| color[index]);
        let mut frame = (count + 1..resource)
            .filter(|&index| anchor[index])
            .collect::<Vec<_>>();
        frame.sort_unstable_by_key(|&index| color[index]);
        let frame = std::iter::once(0)
            .chain(frame.into_iter().map(|index| retained[index - count]))
            .collect();
        let part = part
            .iter()
            .map(|vertex| {
                let Extraction {
                    state: extracted,
                    world,
                    frame,
                } = extract(&state, refinement, &anchor, vertex);
                Part {
                    world: Placement {
                        start: extracted.world.len() - world.len(),
                        original: world,
                    },
                    frame: Placement {
                        start: extracted.frame.len() - frame.len(),
                        original: frame,
                    },
                    search: Search::new(Arc::new(extracted)),
                }
            })
            .collect();
        Some(Self {
            state,
            world,
            frame,
            part,
            current: 0,
        })
    }

    // A step of the part being named, or the whole name once every part has one.
    pub fn step(&mut self) -> Option<Canonical> {
        if let Some(part) = self.part.get_mut(self.current) {
            if part.search.advance() {
                self.current += 1;
            }
            return None;
        }
        let mut named = std::mem::take(&mut self.part)
            .into_iter()
            .map(|part| {
                let name = part
                    .search
                    .finish()
                    .expect("every part is named before the whole");
                (name, part.world, part.frame)
            })
            .collect::<Vec<_>>();
        named.sort_by(|left, right| left.0.state.cmp(&right.0.state));
        let mut world = std::mem::take(&mut self.world);
        let mut frame = std::mem::take(&mut self.frame);
        for (name, own, place) in &named {
            world.extend(own.order(&name.renaming.world));
            frame.extend(place.order(&name.renaming.frame));
        }
        Some(self.state.rename(&world, &frame))
    }
}
