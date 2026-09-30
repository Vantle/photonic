use photonic::runtime::Limit;

pub const LARGE: Limit = Limit {
    configuration: 262_144,
    record: 100_000_000,
    coherence: 1024,
    occurrence: 16_384,
    scope: 2048,
};
