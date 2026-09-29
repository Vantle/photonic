// The hashed index from a part's root and kind to its entries, as the host's table lays it out.
static uint mix(uint value) {
    value ^= value >> 16;
    value *= 0x85ebca6bu;
    value ^= value >> 13;
    value *= 0xc2b2ae35u;
    value ^= value >> 16;
    return value;
}

static bool lookup(device const uint* key, uint mask, uint root, uint kind, thread uint& start, thread uint& number) {
    uint position = mix(root * 0x9e3779b9u ^ mix(kind + 0x7f4a7c15u)) & mask;
    while (true) {
        uint base = position * 4;
        if (key[base] == EMPTY) {
            return false;
        }
        if (key[base] == root && key[base + 1] == kind) {
            start = key[base + 2];
            number = key[base + 3];
            return true;
        }
        position = (position + 1) & mask;
    }
}
