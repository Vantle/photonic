use photonic::runtime::Limit;

pub struct Budget {
    pub work: usize,
    pub bound: Limit,
}

pub const EXPLORATION: Budget = Budget {
    work: 20_000,
    bound: Limit {
        configuration: 128,
        occurrence: 128,
        scope: 16,
        coherence: 16,
        record: 100_000,
    },
};

pub const PATH: Budget = Budget {
    work: 1_000_000,
    bound: Limit {
        configuration: 32768,
        occurrence: 8192,
        scope: 1024,
        coherence: 512,
        record: 2_000_000,
    },
};

pub const SYMMETRY: usize = 100_000;
