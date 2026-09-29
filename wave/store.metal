// Enters every established marking into a larger table.
kernel void rehash(
    device const Arena& arena [[buffer(0)]],
    device const ulong* offset [[buffer(1)]],
    device atomic_uint* table [[buffer(2)]],
    constant Setting& setting [[buffer(3)]],
    uint index [[thread_position_in_grid]]) {
    if (index >= setting.count) {
        return;
    }
    device const uint* marking = locate(arena, offset[index]);
    ulong sum = head(marking[0]);
    for (uint item = 0; item < marking[1]; item++) {
        sum += term(marking[2 + item]);
    }
    ulong mine = scramble(sum);
    uint group = uint(mine) & (setting.bucket - 1);
    while (true) {
        for (uint item = 0; item < WIDTH; item++) {
            device atomic_uint* cell = table + 2 * (ulong(group) * WIDTH + item);
            uint expected = 0;
            while (!atomic_compare_exchange_weak_explicit(cell, &expected, index + 1, memory_order_relaxed, memory_order_relaxed) && expected == 0) {
            }
            if (expected == 0) {
                atomic_store_explicit(cell + 1, uint(mine >> 32) | 1u, memory_order_relaxed);
                return;
            }
        }
        group = (group + 1) & (setting.bucket - 1);
    }
}
