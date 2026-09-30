// The kinds of a marking in order, whether it is stored, written by the host or still a source
// marking without the kinds an entry consumes, merged with the entry's produced kinds; candidates
// are compared and written through it, so a pass never copies a successor it will discard. The
// entry keeps its consumed kinds after the produced ones from the largest down, so the stream holds
// only the kind it waits for and how many it has yet to leave out, and finds the next where the
// count it has left points.
struct Stream {
    device const uint* kind;
    device const uint* made;
    uint length;
    uint count;
    uint consumed;
    uint drop;
    uint left;
    uint right;
    uint root;
    uint size;
};

static Stream stored(device const uint* marking) {
    Stream stream;
    stream.kind = marking + 2;
    stream.made = marking + 2;
    stream.length = marking[1];
    stream.count = 0;
    stream.consumed = 0;
    stream.drop = 0;
    stream.left = 0;
    stream.right = 0;
    stream.root = marking[0];
    stream.size = marking[1];
    return stream;
}

// A candidate is its source, its entry or, for a successor the host found, where its words start
// in the extra memory after its hash, and the ways to choose the copies its entry consumes, with
// flags for joined successors and successors the limits refuse.
static Stream open(uint3 record, device const Arena& arena, device const ulong* offset, device const uint* entry, device const uint* extra) {
    if ((record.z & JOINED) != 0) {
        return stored(extra + record.y);
    }
    device const uint* marking = locate(arena, offset[record.x]);
    device const Header* item = header(entry, record.y);
    Stream stream = stored(marking);
    stream.made = entry + record.y + HEADER;
    stream.count = item->length;
    stream.drop = item->taken;
    stream.consumed = item->taken > 0 ? stream.made[item->length + item->taken - 1] : 0;
    stream.root = item->root;
    stream.size = marking[1] - item->taken + item->length;
    return stream;
}

// A candidate's hash, from its source's sum of kind terms plus what its entry adds, or as the host
// wrote it before a joined successor's words.
static ulong digest(uint3 record, device const ulong* sum, uint first, device const uint* entry, device const uint* extra) {
    if ((record.z & JOINED) != 0) {
        return ulong(extra[record.y - 2]) | (ulong(extra[record.y - 1]) << 32);
    }
    device const Header* item = header(entry, record.y);
    return scramble(sum[record.x - first] + as_type<ulong>(uint2(item->gain)));
}

// The next kind, leaving out a copy of each kind the entry consumes; both lists are sorted, so each
// consumed kind is met where it lies. A stream that leaves out one kind at most is read without the
// loop that leaves out more, which slows the passes' hottest loops even when it never runs.
template <bool Many>
static uint next(thread Stream& stream) {
    if (stream.drop > 0 && stream.left < stream.length && stream.kind[stream.left] == stream.consumed) {
        stream.left += 1;
        stream.drop -= 1;
        while (Many && stream.drop > 0) {
            stream.consumed = stream.made[stream.count + stream.drop - 1];
            if (stream.left >= stream.length || stream.kind[stream.left] != stream.consumed) {
                break;
            }
            stream.left += 1;
            stream.drop -= 1;
        }
    }
    if (stream.right >= stream.count || (stream.left < stream.length && stream.kind[stream.left] <= stream.made[stream.right])) {
        return stream.kind[stream.left++];
    }
    return stream.made[stream.right++];
}

template <bool Many>
static bool alike(Stream left, Stream right) {
    for (uint index = 0; index < left.size; index++) {
        if (next<Many>(left) != next<Many>(right)) {
            return false;
        }
    }
    return true;
}

static bool equal(Stream left, Stream right) {
    if (left.root != right.root || left.size != right.size) {
        return false;
    }
    return left.drop > 1 || right.drop > 1 ? alike<true>(left, right) : alike<false>(left, right);
}

template <bool Many>
static void copy(Stream stream, device uint* out) {
    for (uint index = 0; index < stream.size; index++) {
        out[index] = next<Many>(stream);
    }
}

static void write(Stream stream, device uint* out) {
    out[0] = stream.root;
    out[1] = stream.size;
    if (stream.drop > 1) {
        copy<true>(stream, out + 2);
    } else {
        copy<false>(stream, out + 2);
    }
}
