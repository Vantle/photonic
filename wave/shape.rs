// How much the engine takes on at once: the markings counted together, which bounds the host's
// work on flagged markings; the candidates decided together, which bounds their memory; the words
// of the arena's first segment and of its largest, which bound what a segment commits; and how many
// markings, candidates and table slots the first buffers hold, so small explorations never grow
// them.
#[derive(Clone, Copy, Debug)]
pub struct Shape {
    pub window: usize,
    pub pass: usize,
    pub first: usize,
    pub largest: usize,
    pub initial: usize,
}

impl Default for Shape {
    fn default() -> Self {
        Self {
            window: 1 << 20,
            pass: 1 << 23,
            first: 1 << 20,
            largest: 1 << 26,
            initial: 1 << 16,
        }
    }
}
