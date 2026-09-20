use crate::index::Index;
use smallvec::SmallVec;

pub(super) fn select(index: &Index, changed: &[usize]) -> SmallVec<[usize; 4]> {
    #[cfg(feature = "measurement")]
    let _measurement =
        crate::measurement::profile::Scope::new(crate::measurement::profile::Phase::Context);
    let mut known = vec![None; index.state.frame.len()];
    for &frame in changed {
        if let Some(value) = known.get_mut(frame) {
            *value = Some(true);
        }
    }
    let mut selected = SmallVec::new();
    let mut path = SmallVec::<[usize; 8]>::new();
    for frame in index.frame() {
        let mut owner = Some(frame);
        let affected = loop {
            let Some(current) = owner else {
                break false;
            };
            if let Some(affected) = known[current] {
                break affected;
            }
            path.push(current);
            owner = index.state.frame[current].lexical;
        };
        for current in path.drain(..) {
            known[current] = Some(affected);
        }
        if affected {
            selected.push(frame);
        }
    }
    selected
}
