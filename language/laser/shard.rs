// Workers share a table by splitting it into shards, each behind its own lock or owned by one
// worker at a time. Tables inside a shard place entries by the low bits of the same hash, so shards
// take middle bits that the tables do not use.
pub(super) const COUNT: usize = 64;

pub(super) fn slot(hash: u64) -> usize {
    (hash >> 40) as usize % COUNT
}

pub(super) fn empty<Value: Default>() -> Vec<Value> {
    (0..COUNT).map(|_| Value::default()).collect()
}
