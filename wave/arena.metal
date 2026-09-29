// Markings live in an arena as the root, the number of kinds and the sorted kinds, in segments the
// kernels reach by their device addresses; an offset names a segment in its high half and a word
// within it in its low half.
struct Arena {
    device uint* segment[SEGMENT];
};

static device uint* locate(device const Arena& arena, ulong offset) {
    return arena.segment[offset >> 32] + uint(offset);
}
