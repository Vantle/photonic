# Runtime decisions

The [configuration contract](semantics.md) defines the runtime. The [original grammar](syntax.md) supplies its source notation. These decisions apply equally to ordinary concepts and whole rule values.

## Positive evidence

Every rule lists what it needs. An interaction is enabled by a joint witness supplying those inputs and the rule's availability. Missing evidence never enables an interaction. There are no negative premises, default rules, absence queries, or built-in logical concepts, in source or JSON.

A user may define Not, True, False, or contrary assertions through ordinary rules. The runtime does not assign their labels a truth interpretation or reject their coexistence. Positive evidence remains established even if later events consume the values that supplied it.

## Source projection

A derivation may enable an application at its concrete source. It adds an outgoing event; it does not mutate the past or carry all intermediate evidence forward as extra operands.

Literal outputs consume the projected footprint and preserve unrelated source remainder. A scoped body consumes exact operands and receives the concrete non-exact witness with that remainder. Its continuation records exact consumption. This is how the same abstraction can accept different concrete witnesses without variables or a separate type evaluator.

## Local code

A produced whole rule captures its immutable defining environment. It becomes executable only where it is available alongside its operands. Invocation reads that availability; explicitly matching the rule as an operand consumes it. If an occurrence supplies both roles, consumption wins.

For `[Seed] [A] B` from Seed.A, actual execution produces the rule alongside A and then B. Source inference can also produce Seed.B. If one Seed jointly produces code and operand, consuming the projected operand can consume Seed itself. Competing histories supplying code and data cannot be combined.

Whole-rule replacement is local computation, not global aliasing or behavioral equivalence. Source expressions such as `[[A] B] [A] C` use the same operation as literal replacement. Arbitrary extraction of an unknown rule's fields is not implemented; it requires an encoding through existing primitives.

## Identity and reunion

Coherences are independent contexts with explicit shared introduction identity. Untouched inherited occurrences reconcile once. Independently produced equal values remain distinct occurrences. A joint sibling witness may project back to one source without duplicating that source.

Canonical identity normalizes anonymous introductions and frames while preserving multiplicity, sharing, captures, lexical links, and held continuation resources. A repeated state shares evaluation, but genuinely new evidence may enable more events. Newly allocated names alone do not distinguish states.

Bidirectional broadcast rules can produce growing multiplicities. The runtime does not silently reinterpret them as inverse operations. It deduplicates exact states and permits unbounded genuinely distinct computation.

## Execution and verification

Incremental matching gates retain compatible assignments, not just scalar counts. Different operand positions require distinct occurrences and different coherence positions require compatible distinct coherences. Duplicate arrivals do not complete additional positions. Ordered worker batches preserve deterministic graph mutation.

Positive evidence closure uses indexed premise counts. All required premises must be established before a conclusion is added. A cycle without a fact cannot bootstrap itself. An unchanged snapshot reuses its evidence closure.

The reference suite and Rust tests cover source projection, broadcast and reunion, incompatible histories, multiplicity, code production/replacement, local activation, captures, original-syntax lowering, worker determinism, and suspension. Budgets are implementation limits, not language-level failure facts. Reports are inspection artifacts, not portable proof certificates.

[Obsidian](../mathematics/obsidian.md) checks exact reachability externally. Its finite closed-graph result does not create a negative premise inside a program. The [mathematics plan](../mathematics/plan.md) separates concrete examples from future encoded proof checking and induction.
