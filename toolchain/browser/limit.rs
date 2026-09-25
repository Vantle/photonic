use photonic::runtime::Limit;

pub struct Budget {
    pub work: usize,
    pub bound: Limit,
}

pub const EXPLORATION: Budget = Budget {
    work: 20_000,
    bound: Limit {
        state: 128,
        cell: 128,
        frame: 16,
        world: 16,
        record: 100_000,
    },
};

pub const PATH: Budget = Budget {
    work: 1_000_000,
    bound: Limit {
        state: 32768,
        cell: 8192,
        frame: 1024,
        world: 512,
        record: 2_000_000,
    },
};

pub const SYMMETRY: usize = 100_000;
