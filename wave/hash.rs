// Splitmix's finalizer, a bijection that spreads every bit of a word across the whole word.
pub fn scramble(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

// A kind's term in a marking's sum.
pub fn term(kind: u32) -> u64 {
    scramble(u64::from(kind).wrapping_add(0x9e37_79b9_7f4a_7c15))
}

// A root's term in a marking's sum, scrambled from values no kind's term comes from.
pub fn head(root: u32) -> u64 {
    scramble(u64::from(root) | 1 << 32)
}

// A marking's hash is its scrambled sum of terms, so an event changes the sum by the terms of what
// it removes and adds; the low half picks a table slot and the high half checks it.
pub fn digest(root: u32, kind: &[u32]) -> u64 {
    scramble(
        kind.iter()
            .fold(head(root), |sum, &value| sum.wrapping_add(term(value))),
    )
}
