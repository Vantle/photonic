use photonic::runtime::Limit;

pub struct Budget {
    pub work: usize,
    pub bound: Limit,
}

pub const EXPLORATION: Budget = Budget {
    work: 20_000,
    bound: Limit {
        configuration: 128,
        occurrence: 256,
        scope: 16,
        coherence: 16,
        record: 100_000,
    },
};

// A direct path keeps every configuration it passes, so a path that only grows keeps the square of
// its largest configuration: 8,192 occurrences let one reach 4.3 GB, past the module's 1 GiB, and
// 2,048 keep it near 0.3 GB, while ten-digit products on the calculator hold about 500.
pub const PATH: Budget = Budget {
    work: 1_000_000,
    bound: Limit {
        configuration: 32768,
        occurrence: 2048,
        scope: 1024,
        coherence: 512,
        record: 2_000_000,
    },
};

pub const SYMMETRY: usize = 100_000;
