// A table slot is two words: the first is zero when empty, a marking's id plus one once it is
// established, or a candidate's index with the tag bit while a pass decides which of equal
// candidates becomes the marking; the second is the high half of the established marking's hash.
// The lowest index wins, so numbering never depends on timing and follows the order in which one
// thread searching breadth first would find the markings. A winner's rank and the words its
// marking takes share one running sum, the rank above SHIFT bits and the words below: a pass holds
// fewer than 2^24 candidates, and no device holds 2^40 words.

// Records one successor: its source, its entry and how many copies of the consumed kind could be
// consumed, marked when the limits refuse it; returns how many events it stands for when the limits
// admit it.
static ulong emit(
    device const uint* entry,
    uint cursor,
    uint source,
    uint copy,
    uint3 total,
    uint candidate,
    device const uint* base,
    device packed_uint3* record,
    device atomic_uint* summary,
    constant Setting& setting) {
    device const uint* item = entry + cursor;
    uint3 grown = total + uint3(item[5], item[6], item[7]);
    bool admitted = grown.x <= setting.coherence && base[item[0]] + grown.y <= setting.occurrence && 1 + grown.z <= setting.scope;
    record[candidate] = packed_uint3(source, cursor, admitted ? copy : copy | LIMITED);
    if (!admitted) {
        atomic_store_explicit(&summary[REFUSED], 1u, memory_order_relaxed);
        return 0;
    }
    return ulong(copy) * ulong(item[1]);
}

// Records every successor the tables give a marking, in the order the host's net expands it: the
// events binding only the root, then each run of equal kinds in order; the host writes the
// successors of events joining several components after them. It keeps each marking's sum of kind
// terms, from which its successors' hashes follow, and sums the events the admitted successors
// stand for a threadgroup at a time.
kernel void expand(
    device const Arena& arena [[buffer(0)]],
    device const ulong* offset [[buffer(1)]],
    device const uint* entry [[buffer(2)]],
    device const uint* key [[buffer(3)]],
    device const uint* lone [[buffer(4)]],
    device const uint* size [[buffer(5)]],
    device const uint* base [[buffer(6)]],
    device const ulong* start [[buffer(7)]],
    device packed_uint3* record [[buffer(8)]],
    device ulong* sum [[buffer(9)]],
    device ulong* partial [[buffer(10)]],
    device atomic_uint* summary [[buffer(11)]],
    constant Setting& setting [[buffer(12)]],
    uint index [[thread_position_in_grid]],
    uint member [[thread_index_in_threadgroup]],
    uint group [[threadgroup_position_in_grid]],
    uint lane [[thread_index_in_simdgroup]],
    uint band [[simdgroup_index_in_threadgroup]],
    uint width [[threads_per_simdgroup]],
    uint height [[simdgroups_per_threadgroup]]) {
    threadgroup ulong shared[33];
    ulong weight = 0;
    if (index < setting.count) {
        uint source = setting.first + index;
        device const uint* marking = locate(arena, offset[source]);
        uint root = marking[0];
        uint length = marking[1];
        device const uint* kind = marking + 2;
        ulong own = 0;
        uint3 total = uint3(0);
        for (uint item = 0; item < length; item++) {
            uint value = kind[item];
            own += term(value);
            total += uint3(size[3 * value], size[3 * value + 1], size[3 * value + 2]);
        }
        sum[index] = own;
        uint candidate = uint(start[setting.shift + index] - setting.base);
        uint cursor = lone[2 * root];
        uint number = lone[2 * root + 1];
        for (uint item = 0; item < number; item++) {
            weight += emit(entry, cursor, source, 1, total, candidate, base, record, summary, setting);
            candidate += 1;
            cursor += HEADER + entry[cursor + 2];
        }
        uint position = 0;
        while (position < length) {
            uint value = kind[position];
            uint end = position + 1;
            while (end < length && kind[end] == value) {
                end += 1;
            }
            uint first = 0;
            uint found = 0;
            lookup(key, setting.key - 1, root, value, first, found);
            uint3 rest = total - uint3(size[3 * value], size[3 * value + 1], size[3 * value + 2]);
            cursor = first;
            for (uint item = 0; item < found; item++) {
                weight += emit(entry, cursor, source, end - position, rest, candidate, base, record, summary, setting);
                candidate += 1;
                cursor += HEADER + entry[cursor + 2];
            }
            position = end;
        }
    }
    ulong whole = 0;
    ladder(weight, shared, lane, band, width, height, whole);
    if (member == 0) {
        partial[group] = whole;
    }
}

// Finds each admitted candidate's marking: an established one it equals, or a slot it claims or
// shares with equal candidates. Once the configuration limit admits no new marking, a candidate
// only looks. A bucket is read whole, and a slot seen empty or tagged is read again atomically
// before the candidate claims or joins it. An established marking found no later than its source
// is flagged, since every cycle holds such an edge, and only established markings are found so.
kernel void insert(
    device const Arena& arena [[buffer(0)]],
    device const ulong* offset [[buffer(1)]],
    device const uint* entry [[buffer(2)]],
    device const uint* extra [[buffer(3)]],
    device const packed_uint3* record [[buffer(4)]],
    device const ulong* sum [[buffer(5)]],
    device uint* target [[buffer(6)]],
    device atomic_uint* table [[buffer(7)]],
    device uint* slot [[buffer(8)]],
    device atomic_uint* summary [[buffer(9)]],
    device const uint4* bucket [[buffer(10)]],
    constant Setting& setting [[buffer(11)]],
    uint index [[thread_position_in_grid]]) {
    if (index >= setting.count) {
        return;
    }
    uint3 own = uint3(record[index]);
    if ((own.z & LIMITED) != 0) {
        target[index] = BLOCKED;
        slot[index] = NONE;
        return;
    }
    ulong mine = digest(own, sum, setting.first, entry, extra);
    uint check = uint(mine >> 32) | 1u;
    uint group = uint(mine) & (setting.bucket - 1);
    uint item = 0;
    Stream stream = open(own, arena, offset, entry, extra);
    while (true) {
        device const uint4* line = bucket + 4 * ulong(group);
        uint4 pair[4] = {line[0], line[1], line[2], line[3]};
        uint value = 0;
        uint seen = 0;
        for (; item < WIDTH; item++) {
            uint4 both = pair[item / 2];
            value = item % 2 == 0 ? both.x : both.z;
            seen = item % 2 == 0 ? both.y : both.w;
            if (value == 0 || seen == check || seen == 0) {
                break;
            }
        }
        if (item == WIDTH) {
            group = (group + 1) & (setting.bucket - 1);
            item = 0;
            continue;
        }
        uint position = group * WIDTH + item;
        device atomic_uint* cell = table + 2 * ulong(position);
        if (value == 0) {
            if (setting.allowed == 0) {
                target[index] = BLOCKED;
                slot[index] = NONE;
                atomic_store_explicit(&summary[REFUSED], 1u, memory_order_relaxed);
                return;
            }
            uint expected = 0;
            if (atomic_compare_exchange_weak_explicit(cell, &expected, TAG | index, memory_order_relaxed, memory_order_relaxed)) {
                atomic_store_explicit(cell + 1, check, memory_order_relaxed);
                target[index] = NONE;
                slot[index] = position;
                return;
            }
            if (expected == 0) {
                continue;
            }
            value = expected;
            seen = atomic_load_explicit(cell + 1, memory_order_relaxed);
        }
        if (value < TAG) {
            if (seen == check && equal(stream, stored(locate(arena, offset[value - 1])))) {
                target[index] = value - 1;
                slot[index] = NONE;
                if (value - 1 <= own.x && atomic_load_explicit(&summary[BACKWARD], memory_order_relaxed) == 0) {
                    atomic_store_explicit(&summary[BACKWARD], 1u, memory_order_relaxed);
                }
                return;
            }
        } else if (setting.allowed != 0) {
            uint other = value & ~TAG;
            uint3 theirs = uint3(record[other]);
            bool alike = seen != 0 ? seen == check : digest(theirs, sum, setting.first, entry, extra) == mine;
            if (alike && equal(stream, open(theirs, arena, offset, entry, extra))) {
                if (other > index) {
                    atomic_fetch_min_explicit(cell, TAG | index, memory_order_relaxed);
                }
                target[index] = NONE;
                slot[index] = position;
                return;
            }
        }
        item += 1;
    }
}

// Writes each candidate's share of the pass: the words its marking takes when it owns the slot it
// claimed, and nothing otherwise; and sums each threadgroup's shares, four candidates a thread, with
// the winners' count above SHIFT bits, so a running sum over the groups ranks the winners and places
// their words without a sum for every candidate.
kernel void tally(
    device const Arena& arena [[buffer(0)]],
    device const ulong* offset [[buffer(1)]],
    device const uint* entry [[buffer(2)]],
    device const uint* extra [[buffer(3)]],
    device const packed_uint3* record [[buffer(4)]],
    device atomic_uint* table [[buffer(5)]],
    device const uint* slot [[buffer(6)]],
    device uint* share [[buffer(7)]],
    device ulong* block [[buffer(8)]],
    constant Setting& setting [[buffer(9)]],
    uint index [[thread_position_in_grid]],
    uint member [[thread_index_in_threadgroup]],
    uint group [[threadgroup_position_in_grid]],
    uint lane [[thread_index_in_simdgroup]],
    uint band [[simdgroup_index_in_threadgroup]],
    uint width [[threads_per_simdgroup]],
    uint height [[simdgroups_per_threadgroup]]) {
    threadgroup ulong shared[33];
    ulong mine = 0;
    for (uint item = 0; item < 4; item++) {
        uint candidate = 4 * index + item;
        if (candidate >= setting.count) {
            continue;
        }
        uint position = slot[candidate];
        bool owner = position != NONE && atomic_load_explicit(table + 2 * ulong(position), memory_order_relaxed) == (TAG | candidate);
        uint word = owner ? 2 + open(uint3(record[candidate]), arena, offset, entry, extra).size : 0;
        share[candidate] = word;
        mine += owner ? (1ul << SHIFT) | word : 0;
    }
    ulong whole = 0;
    ladder(mine, shared, lane, band, width, height, whole);
    if (member == 0) {
        block[group] = whole;
    }
}

// The words of the threadgroups before the first whose winners the configuration limit refuses
// all, or of every threadgroup, so the arena never makes room for markings no one writes.
kernel void bound(
    device const ulong* block [[buffer(0)]],
    device ulong* total [[buffer(1)]],
    constant Setting& setting [[buffer(2)]],
    uint index [[thread_position_in_grid]]) {
    if (index > setting.count) {
        return;
    }
    ulong here = index < setting.count ? block[index] : total[WINNER];
    bool reached = index == setting.count || (here >> SHIFT) >= setting.allowed;
    bool before = index > 0 && (block[index - 1] >> SHIFT) >= setting.allowed;
    if (reached && !before) {
        total[BOUND] = here & WORD;
    }
}

// Numbers each winner by its rank, the running sum of its group before it and of the shares before
// it in its group, writes its marking to the arena and establishes its slot. A winner past the
// configuration limit leaves its slot tagged, and nothing looks at tagged slots again. When the
// new markings need more words than the last segment has left, it does nothing, and the host
// places them across a new segment.
kernel void place(
    device const Arena& arena [[buffer(0)]],
    device ulong* offset [[buffer(1)]],
    device const uint* entry [[buffer(2)]],
    device const uint* extra [[buffer(3)]],
    device const packed_uint3* record [[buffer(4)]],
    device uint* target [[buffer(5)]],
    device atomic_uint* table [[buffer(6)]],
    device const uint* slot [[buffer(7)]],
    device const uint* share [[buffer(8)]],
    device const ulong* block [[buffer(9)]],
    device const ulong* total [[buffer(10)]],
    constant Setting& setting [[buffer(11)]],
    uint index [[thread_position_in_grid]],
    uint group [[threadgroup_position_in_grid]],
    uint lane [[thread_index_in_simdgroup]],
    uint band [[simdgroup_index_in_threadgroup]],
    uint width [[threads_per_simdgroup]],
    uint height [[simdgroups_per_threadgroup]]) {
    threadgroup ulong shared[33];
    if (used(total) > setting.room) {
        return;
    }
    ulong mine[4];
    ulong sum = 0;
    for (uint item = 0; item < 4; item++) {
        uint candidate = 4 * index + item;
        uint word = candidate < setting.count ? share[candidate] : 0;
        mine[item] = word != 0 ? (1ul << SHIFT) | word : 0;
        sum += mine[item];
    }
    ulong whole = 0;
    ulong before = block[group] + ladder(sum, shared, lane, band, width, height, whole);
    for (uint item = 0; item < 4; item++) {
        ulong own = mine[item];
        ulong at = before;
        before += own;
        if (own == 0 || (at >> SHIFT) >= setting.allowed) {
            continue;
        }
        uint candidate = 4 * index + item;
        uint aim = setting.next + uint(at >> SHIFT);
        ulong place = at & WORD;
        ulong where = place < setting.split ? setting.arena + place : setting.overflow + (place - setting.split);
        write(open(uint3(record[candidate]), arena, offset, entry, extra), locate(arena, where));
        offset[aim] = where;
        atomic_store_explicit(table + 2 * ulong(slot[candidate]), aim + 1, memory_order_relaxed);
        target[candidate] = aim;
    }
}

// Points every candidate that claimed or shared a slot at the marking its slot established, or
// marks it refused when the configuration limit kept the slot tagged.
kernel void point(
    device uint* target [[buffer(0)]],
    device atomic_uint* table [[buffer(1)]],
    device const uint* slot [[buffer(2)]],
    device atomic_uint* summary [[buffer(3)]],
    device const ulong* total [[buffer(4)]],
    constant Setting& setting [[buffer(5)]],
    uint index [[thread_position_in_grid]]) {
    if (index >= setting.count || used(total) > setting.room) {
        return;
    }
    uint position = slot[index];
    if (position == NONE) {
        return;
    }
    uint value = atomic_load_explicit(table + 2 * ulong(position), memory_order_relaxed);
    if (value < TAG) {
        target[index] = value - 1;
        return;
    }
    target[index] = BLOCKED;
    atomic_store_explicit(&summary[REFUSED], 1u, memory_order_relaxed);
}

// Sums the events the candidates stand for once every candidate points at its marking, a
// threadgroup of candidates at a time and four a thread, leaving out candidates the limits refused;
// a pass the configuration limit may cut short counts its events so, and the host sums the joined
// candidates' events.
kernel void weigh(
    device const uint* entry [[buffer(0)]],
    device const packed_uint3* record [[buffer(1)]],
    device const uint* target [[buffer(2)]],
    device ulong* partial [[buffer(3)]],
    device const ulong* total [[buffer(4)]],
    constant Setting& setting [[buffer(5)]],
    uint index [[thread_position_in_grid]],
    uint member [[thread_index_in_threadgroup]],
    uint group [[threadgroup_position_in_grid]],
    uint lane [[thread_index_in_simdgroup]],
    uint band [[simdgroup_index_in_threadgroup]],
    uint width [[threads_per_simdgroup]],
    uint height [[simdgroups_per_threadgroup]]) {
    threadgroup ulong shared[33];
    if (used(total) > setting.room) {
        return;
    }
    ulong weight = 0;
    for (uint item = 0; item < 4; item++) {
        uint candidate = 4 * index + item;
        if (candidate >= setting.count || target[candidate] == BLOCKED) {
            continue;
        }
        uint3 own = uint3(record[candidate]);
        if ((own.z & JOINED) == 0) {
            weight += ulong(own.z & COPY) * ulong(entry[own.y + 1]);
        }
    }
    ulong whole = 0;
    ladder(weight, shared, lane, band, width, height, whole);
    if (member == 0) {
        partial[group] = whole;
    }
}
