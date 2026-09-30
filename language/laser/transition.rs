use super::layout::{Layout, Site};
use super::makeup::Makeup;
use super::passage::Flat;
use super::taxonomy::Taxonomy;
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

// What applying a key to its parts alone made: the root after it, the kinds it produced, sorted, the
// layouts of the parts before and after, and the passage between them.
pub(super) struct Effect {
    pub root: u32,
    pub produced: Vec<u32>,
    pub source: Layout,
    pub result: Layout,
    pub passage: Flat,
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
        Site::Part(part) => layout.shift(part, &sub, position(part)).place(value),
    };
    let inside = |value: usize| match layout.frame(value) {
        Site::Root => value,
        Site::Part(part) => layout.shift(part, &sub, position(part)).frame(value),
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
                layout.shift(part, &sub, position(part)).world(world)
            })
            .collect(),
        footprint: set(&binding.footprint),
        exact: set(&binding.exact),
        read: set(&binding.read),
        residence: binding.residence,
    };
    Local {
        key: Key {
            root: makeup.root,
            kind,
            frame: inside(frame),
            owner: inside(owner),
            rule,
            binding,
        },
        involved,
    }
}
