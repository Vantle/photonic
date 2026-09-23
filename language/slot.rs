use crate::location::Location;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct Slot {
    pub location: Location,
    pub token: Vec<usize>,
    pub position: usize,
}

pub(crate) fn admits(selection: &[Slot], world: usize) -> bool {
    selection.is_empty()
        || selection
            .iter()
            .any(|slot| slot.location == Location::World(world))
}
