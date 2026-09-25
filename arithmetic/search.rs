use photonic::runtime::Limit;

pub const LIMIT: Limit = Limit {
    state: 4096,
    record: 1_000_000,
    cell: 4096,
    frame: 10,
    world: 4,
};
