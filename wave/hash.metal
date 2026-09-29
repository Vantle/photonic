// Splitmix's finalizer, as the host's hashing::mix.
static ulong scramble(ulong value) {
    value = (value ^ (value >> 30)) * 0xbf58476d1ce4e5b9ul;
    value = (value ^ (value >> 27)) * 0x94d049bb133111ebul;
    return value ^ (value >> 31);
}

static ulong term(uint kind) {
    return scramble(ulong(kind) + 0x9e3779b97f4a7c15ul);
}

static ulong head(uint root) {
    return scramble(ulong(root) | 0x100000000ul);
}
