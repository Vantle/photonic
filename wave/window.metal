// Counts each marking's successors from the tables, those of its joining events included, keeps
// whether its joins are found on the GPU for the pass that expands it, and flags a marking whose
// parts the tables lack, whose joining events need more than the GPU keeps, or which has no
// successor the tables know. A window counted in the command that placed its
// markings covers only the markings that exist once the pass's winners are numbered, and nothing
// when the host must place them.
kernel void count(
    device const Arena& arena [[buffer(0)]],
    device const ulong* offset [[buffer(1)]],
    device const uint* key [[buffer(2)]],
    device const uint* lone [[buffer(3)]],
    device const uint* catalog [[buffer(4)]],
    device const uint* part [[buffer(5)]],
    device const uint* constituent [[buffer(6)]],
    device const uint* coherence [[buffer(7)]],
    device const uint* span [[buffer(8)]],
    device const uint* reach [[buffer(9)]],
    device const uint* rule [[buffer(10)]],
    device const uint* arity [[buffer(11)]],
    device ulong* number [[buffer(12)]],
    device uint2* flagged [[buffer(13)]],
    device atomic_uint* summary [[buffer(14)]],
    device const ulong* total [[buffer(15)]],
    device uint* route [[buffer(16)]],
    constant Setting& setting [[buffer(17)]],
    uint index [[thread_position_in_grid]]) {
    if (index >= setting.count || used(total) > setting.room) {
        return;
    }
    ulong made = ulong(setting.next) + min(total[WINNER] >> SHIFT, ulong(setting.allowed));
    if (ulong(setting.first) + index >= made) {
        number[index] = 0;
        return;
    }
    device const uint* marking = locate(arena, offset[setting.first + index]);
    uint root = marking[0];
    uint length = marking[1];
    device const uint* kind = marking + 2;
    uint successor = 0;
    uint state = 0;
    if (root >= setting.root || lone[2 * root] == EMPTY) {
        state = MISSING;
    } else {
        successor = lone[2 * root + 1];
        uint position = 0;
        while (position < length) {
            uint start = 0;
            uint found = 0;
            if (!lookup(key, setting.key - 1, root, kind[position], start, found)) {
                state = MISSING;
                break;
            }
            successor += found;
            position += run(kind, length, position);
        }
    }
    if (state == 0) {
        Joint joint = {catalog, part, constituent, coherence, span, reach, rule, arity};
        Counter counter = {0};
        state = join(joint, setting, root, kind, length, counter);
        route[index] = state;
        if (state == 0) {
            successor += counter.successor;
        }
        if (state != MISSING && successor == 0) {
            state |= BARE;
        }
    }
    number[index] = successor;
    if (state != 0) {
        uint at = atomic_fetch_add_explicit(&summary[FLAGGED], 1u, memory_order_relaxed);
        flagged[at] = uint2(index, state);
    }
}
