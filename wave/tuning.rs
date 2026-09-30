use crate::setting::OFFER;

// How much the engine takes on at once: the markings counted together, which bounds the host's
// work on flagged markings; the candidates decided together, which bounds their memory; the words
// of the arena's first segment and of its largest, which bound what a segment commits; how many
// markings, candidates and table slots the first buffers hold, so small explorations never grow
// them; and the most offers and parts a marking's joining events are found from on the GPU, at
// most OFFER, past which the host joins it.
#[derive(Clone, Copy, Debug)]
pub struct Tuning {
    pub window: usize,
    pub pass: usize,
    pub first: usize,
    pub largest: usize,
    pub initial: usize,
    pub join: usize,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            window: 1 << 20,
            pass: 1 << 23,
            first: 1 << 20,
            largest: 1 << 26,
            initial: 1 << 16,
            join: OFFER,
        }
    }
}
