// The kinds of a marking in order, whether it is stored, written by the host or still a source
// marking without one copy of a consumed kind, merged with an entry's produced kinds; candidates
// are compared and written through it, so a pass never copies a successor it will discard.
struct Stream {
    device const uint* kind;
    device const uint* made;
    uint length;
    uint count;
    uint consumed;
    uint left;
    uint right;
    uint root;
    uint size;
    bool skip;
};

static Stream stored(device const uint* marking) {
    Stream stream;
    stream.kind = marking + 2;
    stream.made = marking + 2;
    stream.length = marking[1];
    stream.count = 0;
    stream.consumed = 0;
    stream.left = 0;
    stream.right = 0;
    stream.root = marking[0];
    stream.size = marking[1];
    stream.skip = false;
    return stream;
}

// A candidate is its source, its entry or, for a successor the host found, where its words start
// in the extra memory after its hash, and the copies of the consumed kind with flags for joined
// successors and successors the limits refuse.
static Stream open(uint3 record, device const Arena& arena, device const ulong* offset, device const uint* entry, device const uint* extra) {
    if ((record.z & JOINED) != 0) {
        return stored(extra + record.y);
    }
    device const uint* marking = locate(arena, offset[record.x]);
    device const uint* item = entry + record.y;
    Stream stream = stored(marking);
    stream.made = item + HEADER;
    stream.count = item[2];
    stream.consumed = item[8];
    stream.skip = item[8] != LONE;
    stream.root = item[0];
    stream.size = marking[1] - (stream.skip ? 1u : 0u) + item[2];
    return stream;
}

// A candidate's hash, from its source's sum of kind terms less the consumed kind's term plus what
// its entry adds, or as the host wrote it before a joined successor's words.
static ulong digest(uint3 record, device const ulong* sum, uint first, device const uint* entry, device const uint* extra) {
    if ((record.z & JOINED) != 0) {
        return ulong(extra[record.y - 2]) | (ulong(extra[record.y - 1]) << 32);
    }
    device const uint* item = entry + record.y;
    ulong total = sum[record.x - first] + (ulong(item[3]) | (ulong(item[4]) << 32));
    return scramble(item[8] == LONE ? total : total - term(item[8]));
}

static uint next(thread Stream& stream) {
    if (stream.skip && stream.left < stream.length && stream.kind[stream.left] == stream.consumed) {
        stream.skip = false;
        stream.left += 1;
    }
    if (stream.right >= stream.count || (stream.left < stream.length && stream.kind[stream.left] <= stream.made[stream.right])) {
        return stream.kind[stream.left++];
    }
    return stream.made[stream.right++];
}

static bool equal(Stream left, Stream right) {
    if (left.root != right.root || left.size != right.size) {
        return false;
    }
    for (uint index = 0; index < left.size; index++) {
        if (next(left) != next(right)) {
            return false;
        }
    }
    return true;
}

static void write(Stream stream, device uint* out) {
    out[0] = stream.root;
    out[1] = stream.size;
    for (uint index = 0; index < stream.size; index++) {
        out[2 + index] = next(stream);
    }
}
