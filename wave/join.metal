// The tables a marking's joining events are found from: the catalog of parts; each part as its
// root, where its kinds start among the members and how many, and where its entries start and how
// many; each kind's coherences of the root frame and where the inputs it reaches lie; and each
// input's rule and each rule's number of inputs.
struct Joint {
    device const uint* catalog;
    device const uint* part;
    device const uint* member;
    device const uint* coherence;
    device const uint* span;
    device const uint* reach;
    device const uint* rule;
    device const uint* arity;
};

// How many copies of the kind at a run's first position a marking holds.
static uint run(device const uint* kind, uint length, uint position) {
    uint end = position + 1;
    while (end < length && kind[end] == kind[position]) {
        end += 1;
    }
    return end - position;
}

// The ways to take some copies of a kind out of all of them, as the host's net counts them, or some
// number past COPY once there are more than a record holds. Some copies are at most ARITY, so while
// there are at least twice as many copies each step only grows, and fewer copies never pass COPY.
static ulong choose(uint count, uint taken) {
    ulong product = 1;
    for (uint index = 0; index < taken && product <= COPY; index++) {
        product = product * ulong(count - index) / ulong(index + 1);
    }
    return product;
}

// The multisets of a size drawn from so many options, or PICK and one once there are more than
// PICK, which is all a marking's picks may come to on the GPU.
static ulong multiset(uint option, uint size) {
    ulong count = 1;
    for (uint index = 0; index < size && count <= PICK; index++) {
        count = count * ulong(option + index) / ulong(index + 1);
    }
    return min(count, ulong(PICK) + 1);
}

// The next input of a rule after this one with the same offers, or NONE; an input with one offer
// picks it whatever the others pick, so it has none.
static uint follow(thread const uint* offer, thread const uint* start, thread const uint* width, uint inputs, uint item) {
    if (width[item] == 1) {
        return NONE;
    }
    for (uint later = item + 1; later < inputs; later++) {
        bool same = width[later] == width[item];
        for (uint option = 0; option < width[item] && same; option++) {
            same = (offer[start[item] + option] & 0xffffu) == (offer[start[later] + option] & 0xffffu);
        }
        if (same) {
            return later;
        }
    }
    return NONE;
}

// How many picks a rule's inputs make: for each set of inputs with the same offers, the multisets
// of its size drawn from its offers, or PICK and one once there are more than PICK.
static ulong breadth(thread const uint* offer, thread const uint* start, thread const uint* width, uint inputs) {
    ulong count = 1;
    for (uint item = 0; item < inputs && count <= PICK; item++) {
        bool head = width[item] > 1;
        for (uint earlier = 0; earlier < item && head; earlier++) {
            head = follow(offer, start, width, inputs, earlier) != item;
        }
        if (!head) {
            continue;
        }
        uint size = 0;
        for (uint member = item; member != NONE; member = follow(offer, start, width, inputs, member)) {
            size += 1;
        }
        count = min(count * multiset(width[item], size), ulong(PICK) + 1);
    }
    return count;
}

// The number of the part of the root and these kinds, each taken so many times, or NONE when no
// visit grounded it; a slot whose root and digest match is checked kind by kind.
static uint find(Joint joint, uint slots, uint root, ulong digest, device const uint* kind, thread const uint* chosen, thread const uint* taken, uint distinct) {
    uint mask = slots - 1;
    uint position = uint(digest) & mask;
    while (true) {
        device const uint* slot = joint.catalog + 4 * position;
        if (slot[0] == EMPTY) {
            return NONE;
        }
        if (slot[0] == root && slot[1] == uint(digest) && slot[2] == uint(digest >> 32)) {
            device const uint* part = joint.part + PART * slot[3];
            device const uint* member = joint.member + part[1];
            uint at = 0;
            bool same = true;
            for (uint item = 0; item < distinct && same; item++) {
                for (uint copy = 0; copy < taken[item] && same; copy++) {
                    same = at < part[2] && member[at] == kind[chosen[item]];
                    at += 1;
                }
            }
            if (same && at == part[2]) {
                return slot[3];
            }
        }
        position = (position + 1) & mask;
    }
}

// Finds a marking's joining events as the host's net does. Each run of equal kinds offers itself to
// the inputs of joining rules its kind reaches; the offers, sorted by input and then by kind, gather
// each rule's inputs, and a rule whose every input has an offer picks one for each, the first input
// fastest, where an input picks no earlier offer than the next input with the same offers, so
// picks that differ only in which of those took which come once. The picked kinds, each taken as
// many times as inputs picked it, or from once when a copy holds several coherences, up to its
// copies, every way, make a part of the root and two or more components, kept the first time it is
// found. The sink takes each part with the ways to choose its copies. Tells whether the tables
// lacked a part (MISSING) or the marking needs more than the GPU keeps (JOIN), which a marking
// whose rules make more than PICK picks always does; then the sink may have taken only some of
// the parts.
template <typename Sink>
static uint join(Joint joint, constant Setting& setting, uint root, device const uint* kind, uint length, thread Sink& sink) {
    if (setting.rule == 0) {
        return 0;
    }
    uint offer[OFFER];
    uint count = 0;
    uint position = 0;
    while (position < length) {
        uint value = kind[position];
        uint first = joint.span[2 * value];
        uint many = joint.span[2 * value + 1];
        for (uint item = 0; item < many; item++) {
            uint input = joint.reach[first + item];
            if (count == setting.join || input > 0xffffu || position > 0xffffu) {
                return JOIN;
            }
            offer[count] = input << 16 | position;
            count += 1;
        }
        position += run(kind, length, position);
    }
    for (uint index = 1; index < count; index++) {
        uint value = offer[index];
        uint at = index;
        while (at > 0 && offer[at - 1] > value) {
            offer[at] = offer[at - 1];
            at -= 1;
        }
        offer[at] = value;
    }
    uint seen[OFFER];
    uint found = 0;
    uint step = 0;
    ulong pick = 0;
    uint cursor = 0;
    while (cursor < count) {
        uint rule = joint.rule[offer[cursor] >> 16];
        uint start[ARITY];
        uint width[ARITY];
        uint inputs = 0;
        while (cursor < count && joint.rule[offer[cursor] >> 16] == rule) {
            uint input = offer[cursor] >> 16;
            uint end = cursor + 1;
            while (end < count && offer[end] >> 16 == input) {
                end += 1;
            }
            if (inputs == ARITY) {
                return JOIN;
            }
            start[inputs] = cursor;
            width[inputs] = end - cursor;
            inputs += 1;
            cursor = end;
        }
        if (inputs < joint.arity[rule]) {
            continue;
        }
        pick += breadth(offer, start, width, inputs);
        if (pick > PICK) {
            return JOIN;
        }
        uint digit[ARITY];
        for (uint item = 0; item < inputs; item++) {
            digit[item] = 0;
        }
        while (true) {
            uint chosen[ARITY];
            uint filled[ARITY];
            uint distinct = 0;
            for (uint item = 0; item < inputs; item++) {
                uint picked = offer[start[item] + digit[item]] & 0xffffu;
                uint at = 0;
                while (at < distinct && chosen[at] < picked) {
                    at += 1;
                }
                if (at < distinct && chosen[at] == picked) {
                    filled[at] += 1;
                    continue;
                }
                for (uint move = distinct; move > at; move--) {
                    chosen[move] = chosen[move - 1];
                    filled[move] = filled[move - 1];
                }
                chosen[at] = picked;
                filled[at] = 1;
                distinct += 1;
            }
            uint low[ARITY];
            uint high[ARITY];
            uint copies[ARITY];
            bool open = true;
            for (uint item = 0; item < distinct; item++) {
                copies[item] = run(kind, length, chosen[item]);
                low[item] = joint.coherence[kind[chosen[item]]] > 1 ? 1 : filled[item];
                high[item] = min(filled[item], copies[item]);
                open = open && low[item] <= high[item];
            }
            if (open) {
                uint taken[ARITY];
                for (uint item = 0; item < distinct; item++) {
                    taken[item] = low[item];
                }
                while (true) {
                    uint size = 0;
                    for (uint item = 0; item < distinct; item++) {
                        size += taken[item];
                    }
                    if (size >= 2) {
                        step += 1;
                        if (step > STEP) {
                            return JOIN;
                        }
                        ulong sum = head(root);
                        ulong choice = 1;
                        for (uint item = 0; item < distinct; item++) {
                            sum += ulong(taken[item]) * term(kind[chosen[item]]);
                            ulong ways = choose(copies[item], taken[item]);
                            choice = choice > COPY || ways > COPY ? ulong(COPY) + 1 : choice * ways;
                        }
                        uint id = find(joint, setting.catalog, root, scramble(sum), kind, chosen, taken, distinct);
                        if (id == NONE) {
                            return MISSING;
                        }
                        if (choice > COPY) {
                            return JOIN;
                        }
                        bool fresh = true;
                        for (uint item = 0; item < found && fresh; item++) {
                            fresh = seen[item] != id;
                        }
                        if (fresh) {
                            if (found == setting.join) {
                                return JOIN;
                            }
                            seen[found] = id;
                            found += 1;
                            sink.take(joint, id, uint(choice), setting);
                        }
                    }
                    uint item = 0;
                    while (item < distinct && taken[item] == high[item]) {
                        item += 1;
                    }
                    if (item == distinct) {
                        break;
                    }
                    taken[item] += 1;
                    for (uint back = 0; back < item; back++) {
                        taken[back] = low[back];
                    }
                }
            }
            uint item = 0;
            while (item < inputs && digit[item] + 1 == width[item]) {
                item += 1;
            }
            if (item == inputs) {
                break;
            }
            digit[item] += 1;
            for (uint back = item; back > 0; back--) {
                uint next = follow(offer, start, width, inputs, back - 1);
                digit[back - 1] = next == NONE ? 0 : digit[next];
            }
        }
    }
    return 0;
}

// Counts the successors of a marking's joining events: one for every entry of every part.
struct Counter {
    uint successor;

    void take(Joint joint, uint id, uint, constant Setting&) {
        successor += joint.part[PART * id + 4];
    }
};
