use crate::incidence::{Incidence, Label};
use crate::link::Link;
use crate::state::{Canonical, State};
use smallvec::SmallVec;

pub(super) fn rename(
    state: &State,
    incidence: &Incidence,
    world: &[usize],
    frame: &[usize],
) -> Canonical {
    #[cfg(feature = "measurement")]
    let _measurement =
        crate::measurement::profile::Scope::new(crate::measurement::profile::Phase::Renaming);
    state.remap(world, frame, |_| {
        let mut position = vec![0; incidence.label.len() - incidence.resource.len()];
        for (ordinal, &index) in world.iter().enumerate() {
            position[index] = ordinal;
        }
        for (ordinal, &index) in frame.iter().enumerate() {
            position[incidence.frame[index]] = ordinal;
        }
        let start = position.len();
        let mut resource = incidence
            .resource
            .iter()
            .enumerate()
            .map(|(offset, &identity)| {
                let vertex = start + offset;
                let Label::Resource(symbol) = incidence.label[vertex] else {
                    unreachable!()
                };
                let mut capture = None;
                let mut membership = SmallVec::<[(u8, usize); 1]>::new();
                for &(kind, target) in &incidence.edge[vertex] {
                    let kind = match kind {
                        value if value == Link::Particle as u8 => 0,
                        value if value == Link::Holder as u8 => 1,
                        value if value == Link::Owner as u8 => 2,
                        value if value == Link::Capture as u8 => {
                            capture = Some(position[target]);
                            continue;
                        }
                        _ => unreachable!(),
                    };
                    membership.push((kind, position[target]));
                }
                membership.sort_unstable();
                (identity, (symbol, capture, membership))
            })
            .collect::<Vec<_>>();
        resource.sort_by(|left, right| left.1.cmp(&right.1).then_with(|| left.0.cmp(&right.0)));
        resource
            .iter()
            .enumerate()
            .map(|(ordinal, (identity, _))| (*identity, ordinal))
            .collect()
    })
}
