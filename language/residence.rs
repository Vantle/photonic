use crate::place::Place;

// Where the rule an event applies was when it matched: live in a frame's context, or a value a
// coherence holds.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum Residence {
    Context,
    World,
}

impl Residence {
    // A held occurrence is evidence a frame keeps, never a rule an event reads.
    pub fn of(place: Place) -> Option<Self> {
        match place {
            Place::Context(..) => Some(Self::Context),
            Place::World(..) => Some(Self::World),
            Place::Held(..) => None,
        }
    }
}
