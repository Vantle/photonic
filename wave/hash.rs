// A kind's term in a marking's sum.
pub fn term(kind: u32) -> u64 {
    hashing::mix(u64::from(kind).wrapping_add(0x9e37_79b9_7f4a_7c15))
}

// A root's term in a marking's sum, mixed from values no kind's term comes from.
pub fn head(root: u32) -> u64 {
    hashing::mix(u64::from(root) | 1 << 32)
}

// The sum of a root's and its kinds' terms, so an event changes a marking's sum by the terms of
// what it removes and adds.
pub fn sum(root: u32, kind: &[u32]) -> u64 {
    kind.iter()
        .fold(head(root), |sum, &value| sum.wrapping_add(term(value)))
}

// A marking's hash is its mixed sum; the low half picks a table slot and the high half checks it.
pub fn digest(root: u32, kind: &[u32]) -> u64 {
    hashing::mix(sum(root, kind))
}
