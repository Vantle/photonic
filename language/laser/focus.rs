use super::Identity;
use super::enclosure::Enclosure;
use super::independence::Independence;
use crate::application::Owner;
use crate::place::Place;
use crate::state::State;

// A reduced exploration fires, at a configuration, only the events of one part whose events commute
// with every other event, now and after any events outside it: a coherence that only events binding
// it alone can touch, or a sealed scope. Such events stay enabled until they fire, so every
// configuration where a run ends is still reached, and a run can go on forever exactly when the
// reduced exploration has a cycle. A reduced exploration is plain and carries no trace back, so
// every rule it applies lives in a frame of the configuration it fires at, never in a capture that
// is gone.
pub(super) enum Focus {
    World(usize),
    Scope(Enclosure),
}

impl Focus {
    pub fn choose(
        state: &State,
        identity: &[&Identity],
        independence: &Independence,
    ) -> Option<Self> {
        let mut world = vec![0; state.world.len()];
        for identity in identity {
            for &index in &identity.binding.world {
                world[index] += 1;
            }
        }
        let mut candidate = world
            .into_iter()
            .enumerate()
            .filter(|&(_, count)| count > 0)
            .map(|(index, count)| (count, Self::World(index)))
            .collect::<Vec<_>>();
        for frame in 1..state.frame.len() {
            let enclosure = Enclosure::new(state, frame);
            let count = identity
                .iter()
                .filter(|identity| enclosure.holds(identity.frame))
                .count();
            if count > 0 {
                candidate.push((count, Self::Scope(enclosure)));
            }
        }
        candidate.sort_by_key(|(count, focus)| (focus.waiting(state), *count, focus.rank()));
        candidate
            .into_iter()
            .map(|(_, focus)| focus)
            .find(|focus| match focus {
                Self::World(index) => isolated(state, *index, identity, independence),
                Self::Scope(enclosure) => enclosure.sealed(state),
            })
    }

    pub fn holds(&self, identity: &Identity) -> bool {
        match self {
            Self::World(index) => identity.binding.world.contains(index),
            Self::Scope(enclosure) => enclosure.holds(identity.frame),
        }
    }

    // Coherences of the root wait until the scopes have nothing left to do, so a run finishes what it
    // started before it starts more, and configurations stay small.
    fn waiting(&self, state: &State) -> bool {
        matches!(self, Self::World(index) if state.world[*index].frame == 0)
    }

    fn rank(&self) -> (bool, usize) {
        match self {
            Self::World(index) => (false, *index),
            Self::Scope(enclosure) => (true, enclosure.scope()),
        }
    }
}

// A coherence is isolated when every event binding it binds it alone, takes and reads nothing else
// but live rules, and does not return from its scope, when no event of its frame binds nothing, so
// consuming it cannot close the frame under another event, and when no rule could join it.
fn isolated(
    state: &State,
    world: usize,
    identity: &[&Identity],
    independence: &Independence,
) -> bool {
    let frame = state.world[world].frame;
    let alone = identity
        .iter()
        .filter(|identity| identity.binding.world.contains(&world))
        .all(|identity| {
            let returning = identity.frame != 0 && identity.owner == Owner::Frame(identity.frame);
            let within = identity
                .binding
                .footprint
                .iter()
                .chain(&identity.binding.exact)
                .chain(&identity.binding.read)
                .all(|place| match place {
                    Place::World(index, _) => *index == world,
                    Place::Context(..) => true,
                    Place::Held(..) => false,
                });
            identity.binding.world.len() == 1 && !returning && within
        });
    let empty = identity
        .iter()
        .any(|identity| identity.frame == frame && identity.binding.world.len() == 0);
    alone && !empty && !independence.joinable(&state.world[world])
}
