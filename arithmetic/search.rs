use photonic::runtime::Limit;

pub const LIMIT: Limit = Limit {
    configuration: 4096,
    record: 1_000_000,
    occurrence: 4096,
    scope: 10,
    coherence: 4,
};
