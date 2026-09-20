# Persistent incremental evaluation

The subsequent [optimization audit](optimization.md) records dependency-driven scheduling, bounded join replay, faster posting intersections, tighter module interfaces, and the remaining unimplemented recommendations.

The native path evaluator maintains its matching network across rewrites. Rules with the same input share a compiled plan and a lazy matcher. A transition reports an explicit change, which updates world membership, affected matching domains, resource accounting, reachability, and fingerprints. Large state collections share persistent tree nodes with their historical versions.

This is an in-process Rust implementation over Photonic's typed incidence graph. World membership, resource identity, frame relationships, captures, and rule read dependencies remain distinct relationships. Synchronization still requires a complete valid gate binding. Storage positions and traversal order are implementation details; they do not impose an execution order on Photonic.

## Runtime boundaries

| Responsibility | Implementation |
| --- | --- |
| Compile and intern rule inputs | [plan.rs](../language/plan.rs), [catalog.rs](../language/catalog.rs) |
| Maintain eligible plans and rule consumers | [dispatch.rs](../language/dispatch.rs) |
| Enumerate synchronized bindings lazily | [joining.rs](../language/joining.rs), [particle.rs](../language/particle.rs), [assignment.rs](../language/assignment.rs) |
| Index worlds, symbols, captures, and executable rule tokens | [index.rs](../language/index.rs), [reader.rs](../language/reader.rs) |
| Maintain stable sites and current world positions | [position.rs](../language/position.rs), [membership.rs](../language/membership.rs) |
| Compile output construction and perform a rewrite | [recipe.rs](../language/recipe.rs), [rewrite.rs](../language/rewrite.rs) |
| Describe changed worlds and frames | [change.rs](../language/change.rs) |
| Share immutable state and fingerprint collections | [sequence.rs](../language/sequence.rs) |
| Maintain reachability and resource accounting | [reachability.rs](../language/reachability.rs), [layout.rs](../language/layout.rs) |
| Filter state comparisons and resolve collisions exactly | [fingerprint.rs](../language/fingerprint.rs), [structure.rs](../language/structure.rs), [canonical.rs](../language/canonical.rs) |
| Retain inspectable states and transition provenance | [path.rs](../language/path.rs) |

The exhaustive proof evaluator retains its separate synchronization, inference, and provenance machinery in `gate.rs`, `search.rs`, `application.rs`, and `runtime.rs`. It shares state storage and world indexing with the path evaluator and serves as an independent reference for the new direct matcher and rewrite implementation. The superseded path activation and query caches have been removed.

## Matching

Each interned input plan has symbol dependencies. The world index reports net changes to global symbol presence, rather than notifying the dispatcher every time an unchanged symbol is removed and reinserted. The dispatcher updates plan eligibility and affected frame consumers. Frame occupancy, lexical context changes, and executable rule tokens participate in invalidation.

A matching entry owns candidate domains and reusable particle matchers. Unchanged domains survive transitions. A removed site always loses its particle cache, even when an insertion immediately reuses the same site number. Capture-free inputs share matching across lexical owners; capture-dependent inputs retain separate owner keys. Every consumer retains its own rule, owner, and read dependency.

Domains are visited in increasing cardinality. A cheap distinct-world assignment check accepts easy cases, with an exact bipartite matching fallback. Token enumeration remains lazy. Identical input positions use the existing symmetry restriction, and results are restored to input-position order. Inline storage spills to the heap; it does not limit gate width.

Preparation and reuse statistics now count shared matching entries rather than repeated requests by individual rule consumers. Their counts are not directly comparable to the previous per-request cache statistics.

The runtime stores factorized domains, not an eagerly materialized Cartesian product. It retains particle candidate preparation across rewrites, but does not memoize every complete binding or claim a general worst-case-optimal join algorithm. The choice follows the evidence behind hybrid approaches such as [Free Join](https://arxiv.org/abs/2301.10841): the best execution strategy depends on query structure. Delta maintenance follows the general approach of [DBSP](https://docs.feldera.com/vldb23.pdf), without substituting database set semantics for Photonic resource semantics.

## State updates and history

A rewrite identifies removed source worlds, appended target worlds, and changed frame slots. Compiled output recipes retain output scopes and capture requirements. Remainders and enclosed resources are prepared once and shared across output construction. The implementation applies the same generic machinery to arithmetic and every other program.

Stable world sites are distinct from observable world positions. Small indexes use a dense array; larger indexes use a Fenwick tree for logarithmic rank and selection, with amortized compaction of tombstones. The index returns to dense storage after substantial shrinkage. Surviving worlds no longer need to be renumbered after every deletion. Frame membership uses compact arrays up to 32 sites and a tree beyond that; small frames avoid tree allocation on each transition.

State collections use flat storage up to 256 elements and then the public [`imbl` persistent vector](https://docs.rs/imbl/latest/imbl/vector/index.html), pinned to 7.0.2 in `Cargo.lock`. Its RRB tree supports structural sharing and logarithmic edits. The threshold was selected after measuring the overhead of tree access on arithmetic's smaller states. Range access seeks directly to the requested interval instead of walking the preceding elements. Equality, ordering, and hashing depend on logical contents, regardless of storage representation.

Reachability retains counts of references from worlds and captures, with the root permanently included. If the root set and frame structure are unchanged, the reachable-frame result is shared. Other changes use full reachability, including cyclic frame relationships. This avoids the incorrect assumption that ordinary reference counting can collect every graph.

When frame structure is unchanged, world fingerprint contributions are removed and inserted into an unordered accumulator. Frame hashes and untouched world hashes are shared. Frame changes use the existing full frame pass with reusable unchanged contributions. Cell counts and the next resource identifier use the explicit change, with full recomputation when removing the highest resource requires it. Hash equality remains a filter followed by exact comparison where necessary.

Historical states retain complete immutable contents through structural sharing. Every executed transition still has its footprint, exact resources, and read dependency. This is structural reuse, not partial-order pruning: no execution is discarded based on an independence assumption. [Unfolding-based partial-order reduction](https://arxiv.org/abs/1507.00980) could reduce some concurrent search spaces much further, but applying it here would require preserving the inspectable histories and backward proof dependencies as well as reachability.

## Complexity and limits

For the retention benchmark's family of `R` rules sharing an unchanged impossible input over `T` transitions, the previous scheduler repeatedly visited those `R` rule consumers. The new scheduler prepares their shared matcher once. The redundant portion changes from approximately `O(T × R)` work to `O(R + T)`, including initial construction. If those rules actually match, delivering all their distinct results still requires work proportional to the number of results.

Persistent collections, order statistics, and unchanged-frame fingerprint updates make several state operations depend on the changed region instead of the entire world collection. This is not a logarithmic bound for a whole runtime transition: posting maintenance, affected domain updates, lexical topology changes, reachability fallback, graph refinement, and exact canonicalization can still require larger passes.

There is no arithmetic shortcut, change to the Photonic program library, frontend semantic change, or relaxation of browser resource defaults. Thirty factors of two still execute 151,476 transitions. Large gains on retained rules and state do not imply an exponential or thousand-fold arithmetic speedup.

## Verification

All 108 Bazel test targets pass, including native programs, exhaustive reference snapshots, browser conformance, and build-tool checks. Formatting and lint checks pass. Native and WebAssembly builds use the locked Bazel dependency graph; Linux and Windows execution was not tested on this macOS host.

Additional coverage includes:

- Complete binding and provenance sets compared with the independent exhaustive evaluator.
- Cached matching compared with fresh matching across successive rewrites.
- 1,024 generated join mutations, including capture changes and site reuse, plus an eight-world gate.
- 1,024 reachability mutations with cyclic frame links, compared with full traversal.
- 8,192 order-statistic mutations with rank/select checks and compaction.
- Persistent snapshots, range access, and equal hashing across flat and tree representations.
- Occupancy changes, read-token replacement, shared rule inputs, multiplicity, and capture-specific owners.
- Exact layout and fingerprint comparisons with full recomputation, and preparation counts independent of 128 versus 8,192 unrelated rule consumers.

The saved browser demo records changed only in their runtime work counters. Their inputs, results, transition counts, and displayed proof events were checked for exact equality before the records were updated.

## Measurement

Measurements compare `1b99d02` with this implementation on an Apple M5 Max running macOS 26.6.2. Both revisions use the identical benchmark harness, `bazel run -c opt`, one warmup, and five measured samples. Each case is measured as a before/after pair, alternating which revision runs first. Runs are sequential without concurrent builds or tests. Raw samples, arguments, initialization times, work counts, and transition counts are in [rewrite.json](rewrite.json).

Execution includes search and establishing the result. Parsing, program construction, reporting, and release are outside that timer. Runtime initialization is measured separately; the table also reports speedup for initialization plus execution. These are medians, not universal performance guarantees.

| Program | Before (ms) | After (ms) | Execution speedup | Including setup |
| --- | ---: | ---: | ---: | ---: |
| Six factors of two | 66.998 | 68.882 | 0.97× | 0.98× |
| Ten factors of two | 186.775 | 194.099 | 0.96× | 0.96× |
| Twenty factors of two | 1,034.604 | 1,062.747 | 0.97× | 0.97× |
| Thirty factors of two | 3,102.193 | 3,185.446 | 0.97× | 0.97× |
| Nested ternary expression | 52.395 | 53.730 | 0.98× | 0.98× |
| Ten-digit decimal addition | 196.877 | 194.799 | 1.01× | 1.01× |
| Five-digit decimal multiplication | 222.884 | 220.305 | 1.01× | 1.01× |
| 200 retained rule consumers | 13.788 | 2.270 | 6.07× | 3.88× |
| 2,000 retained rule consumers | 48.740 | 1.360 | 35.84× | 16.47× |
| 20,000 retained rule consumers | 363.391 | 1.415 | 256.73× | 29.82× |
| 16 untouched worlds | 1.367 | 1.875 | 0.73× | 0.72× |
| 1,000 untouched worlds | 10.864 | 2.974 | 3.65× | 2.70× |
| 10,000 untouched worlds | 106.547 | 16.564 | 6.43× | 4.30× |

Values below 1× indicate a slowdown. Arithmetic remains within about 4% of baseline in this run and is not uniformly faster. The tiny storage workload increases from 1.37 ms to 1.87 ms; maintaining the extra indexes has a cost when little state can be reused. The large improvements apply to the explicitly measured retention and larger-storage workloads.

The retention workload contains one unmatched `[A,A]` input shared by many distinct rules, plus a chain of 1,000 enabled transitions. The storage workload retains distinct idle worlds while another world advances through 1,000 transitions. Arithmetic measurements execute the existing ternary programs. Successful transition counts agree between revisions in every case.

Reproduce representative cases with:

```sh
bazel run -c opt //benchmark:expression -- '2*2*2*2*2*2' 2101 --sample 5
bazel run -c opt //benchmark:arithmetic -- 1234567890 add 9876543210 --radix 10 --sample 5
bazel run -c opt //benchmark:arithmetic -- 12345 multiply 67890 --radix 10 --sample 5
bazel run -c opt //benchmark:retention -- --width 20000 --length 1000 --sample 5
bazel run -c opt //benchmark:storage -- --width 10000 --length 1000 --sample 5
bazel test -c opt --nocache_test_results //... //toolchain:check //toolchain/browser:check
bazel build -c opt --config=format //...
bazel build -c opt --config=lint //...
```

Earlier measurements remain available in [incremental.json](incremental.json) and [refactoring.json](refactoring.json).
