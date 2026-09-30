#include <metal_stdlib>
using namespace metal;

// The host declares every constant the kernels share with it ahead of these sources, from its own
// values, and joins the sources in the order they declare what later ones use.

struct Setting {
    ulong base;
    ulong arena;
    ulong split;
    ulong overflow;
    ulong room;
    uint first;
    uint count;
    uint shift;
    uint key;
    uint root;
    uint catalog;
    uint rule;
    uint join;
    uint coherence;
    uint occurrence;
    uint scope;
    uint bucket;
    uint next;
    uint allowed;
};

struct Span {
    uint count;
    uint add;
    uint slot;
};

// The words the pass's admitted winners take at most: every winner's words, or fewer once the
// configuration limit can cut the pass short and the bound kernel has left out winners it refuses.
static ulong used(device const ulong* total) {
    return min(total[BOUND], total[WINNER] & WORD);
}
