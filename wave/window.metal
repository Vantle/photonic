// Counts each marking's successors from the tables, and flags a marking whose parts the tables
// lack, whose kinds could fill every input of a rule joining several coherences, or which has no
// successor the tables know. A window counted in the command that placed its markings covers only
// the markings that exist once the pass's winners are numbered, and nothing when the host must
// place them.
kernel void count(
    device const Arena& arena [[buffer(0)]],
    device const ulong* offset [[buffer(1)]],
    device const uint* key [[buffer(2)]],
    device const uint* lone [[buffer(3)]],
    device const ulong* mask [[buffer(4)]],
    device const ulong* need [[buffer(5)]],
    device ulong* number [[buffer(6)]],
    device uint2* flagged [[buffer(7)]],
    device atomic_uint* summary [[buffer(8)]],
    device const ulong* total [[buffer(9)]],
    constant Setting& setting [[buffer(10)]],
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
    ulong have = 0;
    if (root >= setting.root || lone[2 * root] == EMPTY) {
        state = MISSING;
    } else {
        successor = lone[2 * root + 1];
        uint position = 0;
        while (position < length) {
            uint value = kind[position];
            uint end = position + 1;
            while (end < length && kind[end] == value) {
                end += 1;
            }
            uint start = 0;
            uint found = 0;
            if (!lookup(key, setting.key - 1, root, value, start, found)) {
                state = MISSING;
                break;
            }
            successor += found;
            have |= mask[value];
            position = end;
        }
    }
    if (state == 0) {
        if (setting.wide != 0 && have != 0) {
            state |= JOIN;
        }
        for (uint rule = 0; rule < setting.rule && (state & JOIN) == 0; rule++) {
            if ((have & need[rule]) == need[rule]) {
                state |= JOIN;
            }
        }
        if (successor == 0) {
            state |= BARE;
        }
    }
    number[index] = successor;
    if (state != 0) {
        uint at = atomic_fetch_add_explicit(&summary[FLAGGED], 1u, memory_order_relaxed);
        flagged[at] = uint2(index, state);
    }
}
