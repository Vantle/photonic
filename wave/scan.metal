// A SIMD group's inclusive running sum; the SIMD functions take no 64-bit values, so each value
// moves as two halves.
static ulong climb(ulong value, uint lane, uint width) {
    for (uint delta = 1; delta < width; delta <<= 1) {
        uint2 pair = as_type<uint2>(value);
        uint2 other = uint2(simd_shuffle_up(pair.x, ushort(delta)), simd_shuffle_up(pair.y, ushort(delta)));
        if (lane >= delta) {
            value += as_type<ulong>(other);
        }
    }
    return value;
}

// A threadgroup's exclusive running sum, with the group's whole sum in total. Every thread of the
// group calls it, and the group is height whole SIMD groups of width lanes, with height at most
// width, so one SIMD group sums the others.
static ulong ladder(ulong value, threadgroup ulong* shared, uint lane, uint band, uint width, uint height, thread ulong& total) {
    ulong inclusive = climb(value, lane, width);
    if (lane == width - 1) {
        shared[band] = inclusive;
    }
    threadgroup_barrier(mem_flags::mem_threadgroup);
    if (band == 0) {
        ulong own = lane < height ? shared[lane] : 0;
        ulong prefix = climb(own, lane, width);
        if (lane < height) {
            shared[lane] = prefix - own;
        }
        if (lane == width - 1) {
            shared[32] = prefix;
        }
    }
    threadgroup_barrier(mem_flags::mem_threadgroup);
    total = shared[32];
    return shared[band] + inclusive - value;
}

// Each threadgroup sums a tile of four values a thread.
kernel void reduce(
    device const ulong* value [[buffer(0)]],
    device ulong* partial [[buffer(1)]],
    constant Span& span [[buffer(2)]],
    uint member [[thread_index_in_threadgroup]],
    uint size [[threads_per_threadgroup]],
    uint group [[threadgroup_position_in_grid]],
    uint lane [[thread_index_in_simdgroup]],
    uint band [[simdgroup_index_in_threadgroup]],
    uint width [[threads_per_simdgroup]],
    uint height [[simdgroups_per_threadgroup]]) {
    threadgroup ulong shared[33];
    ulong begin = (ulong(group) * size + member) * 4;
    ulong sum = 0;
    for (uint item = 0; item < 4; item++) {
        if (begin + item < span.count) {
            sum += value[begin + item];
        }
    }
    ulong total = 0;
    ladder(sum, shared, lane, band, width, height, total);
    if (member == 0) {
        partial[group] = total;
    }
}

// Turns each tile into its exclusive running sum, starting from the tile's partial when the tiles
// below were summed; a pass with one tile writes the whole sum to its total slot.
kernel void spread(
    device ulong* value [[buffer(0)]],
    device const ulong* partial [[buffer(1)]],
    device ulong* total [[buffer(2)]],
    constant Span& span [[buffer(3)]],
    uint member [[thread_index_in_threadgroup]],
    uint size [[threads_per_threadgroup]],
    uint group [[threadgroup_position_in_grid]],
    uint lane [[thread_index_in_simdgroup]],
    uint band [[simdgroup_index_in_threadgroup]],
    uint width [[threads_per_simdgroup]],
    uint height [[simdgroups_per_threadgroup]]) {
    threadgroup ulong shared[33];
    ulong begin = (ulong(group) * size + member) * 4;
    ulong item[4];
    ulong sum = 0;
    for (uint index = 0; index < 4; index++) {
        item[index] = begin + index < span.count ? value[begin + index] : 0;
        sum += item[index];
    }
    ulong whole = 0;
    ulong prefix = ladder(sum, shared, lane, band, width, height, whole);
    if (span.add != 0) {
        prefix += partial[group];
    }
    for (uint index = 0; index < 4; index++) {
        if (begin + index < span.count) {
            value[begin + index] = prefix;
        }
        prefix += item[index];
    }
    if (span.add == 0 && member == 0) {
        total[span.slot] = whole;
    }
}
