use super::layout::{Layout, Site};
use super::passage::Passage;
use super::taxonomy::{Makeup, Taxonomy};
use crate::basis::Set;
use crate::flow::Binding;
use crate::place::Place;
use smallvec::SmallVec;

// An event reads and changes only the components holding what it binds, the frame where it fires,
// the frame holding its rule, and the root; applied to those alone, with the others kept as they
// are, it gives the whole result. So an event is keyed by the root, the kinds it touches in order
// and its binding laid out over them alone, and applying a key once serves every configuration
// where it recurs. A result whose root ties to a component is named whole instead.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(super) struct Key {
    pub root: u32,
    pub kind: SmallVec<[u32; 4]>,
    pub frame: usize,
    pub owner: usize,
    pub rule: usize,
    pub binding: Binding,
}

pub(super) struct Local {
    pub key: Key,
    pub involved: SmallVec<[usize; 4]>,
}

pub(super) enum Transition {
    Whole,
    Local(Box<Effect>),
}

pub(super) struct Effect {
    pub root: u32,
    pub produced: Vec<u32>,
    pub source: Layout,
    pub result: Layout,
    pub passage: Passage,
}

pub(super) fn involve(
    layout: &Layout,
    frame: usize,
    owner: usize,
    binding: &Binding,
) -> SmallVec<[usize; 4]> {
    let mut involved = SmallVec::new();
    involved.extend(binding.world.iter().map(|&world| layout.world(world)));
    for &place in binding
        .footprint
        .iter()
        .chain(&binding.exact)
        .chain(&binding.read)
    {
        if let Site::Part(part) = layout.site(place) {
            involved.push(part);
        }
    }
    for index in [frame, owner] {
        if let Site::Part(part) = layout.frame(index) {
            involved.push(part);
        }
    }
    involved.sort_unstable();
    involved.dedup();
    involved
}

pub(super) fn localize(
    taxonomy: &Taxonomy,
    layout: &Layout,
    makeup: &Makeup,
    frame: usize,
    owner: usize,
    rule: usize,
    binding: &Binding,
) -> Local {
    let involved = involve(layout, frame, owner, binding);
    let kind = involved
        .iter()
        .map(|&part| makeup.kind[part])
        .collect::<SmallVec<[u32; 4]>>();
    let sub = Layout::of(taxonomy, makeup.root, &kind);
    let position = |part: usize| {
        involved
            .binary_search(&part)
            .expect("every touched part is involved")
    };
    let relocate = |value: Place| match layout.site(value) {
        Site::Root => value,
        Site::Part(part) => layout.move_place(value, part, &sub, position(part)),
    };
    let shift = |value: usize| match layout.frame(value) {
        Site::Root => value,
        Site::Part(part) => layout.move_frame(value, part, &sub, position(part)),
    };
    let set = |value: &Set<Place>| {
        value
            .iter()
            .map(|&value| relocate(value))
            .collect::<Set<_>>()
    };
    let binding = Binding {
        world: binding
            .world
            .iter()
            .map(|&world| {
                let part = layout.world(world);
                layout.move_world(world, part, &sub, position(part))
            })
            .collect(),
        footprint: set(&binding.footprint),
        exact: set(&binding.exact),
        read: set(&binding.read),
    };
    Local {
        key: Key {
            root: makeup.root,
            kind,
            frame: shift(frame),
            owner: shift(owner),
            rule,
            binding,
        },
        involved,
    }
}
