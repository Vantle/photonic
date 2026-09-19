# Incremental evaluation

The runtime maintains typed hypergraph structure and candidate membership directly in Rust. It uses delta maintenance and shared partial results without introducing a database server, query language, native arithmetic shortcuts, or an execution order for Photonic. The frontend and browser resource defaults remain unchanged.

## Module boundaries

The implementation separates immutable descriptions, mutable indexes, and search-local work. All modules below are private to the Rust library; the public runtime and frontend interfaces stay the same.

| Responsibility | Module | Ownership |
| --- | --- | --- |
| Symbol and capture matching | [term.rs](../language/term.rs) | One predicate shared by particle enumeration and selection updates; atom captures are ignored, rule captures must agree. |
| Input compilation | [plan.rs](../language/plan.rs) | Immutable normalized patterns and owner-dependent capture binding. |
| Rule dependencies | [catalog.rs](../language/catalog.rs) | Interned input plans, rule-to-plan mapping, and revisions driven by changed symbols or occupancy. |
| Candidate caching | [query.rs](../language/query.rs) | Selection lifetime, generation checks, and preparation/reuse accounting. |
| Candidate membership | [selection.rs](../language/selection.rs), [index.rs](../language/index.rs) | Stable sites, candidate domains, and incremental insertion/retraction. |
| Gate synchronization | [gate.rs](../language/gate.rs) | Candidate arrival and continuation scheduling, distinct-world constraints, and equivalent-pattern symmetry. |
| Partial binding storage | [prefix.rs](../language/prefix.rs), [slot.rs](../language/slot.rs) | Arena allocation and ancestor traversal; complete bindings alone become flat vectors. |
| Relationship vocabulary | [link.rs](../language/link.rs) | Explicit paired edge roles shared by full and incremental incidence construction. |
| State graph lifecycle | [structure.rs](../language/structure.rs) | Vertex retention, frame preparation, relationship installation, and unreachable-vertex reclamation. |
| Refinement updates | [propagation.rs](../language/propagation.rs) | Dirty propagation, per-round accumulators, and dense-pass fallback. |
| Hash primitives | [hashing.rs](../language/hashing.rs), [accumulator.rs](../language/accumulator.rs) | Shared scalar mixing and unordered collection aggregation. |
| State comparison | [fingerprint.rs](../language/fingerprint.rs), [canonical.rs](../language/canonical.rs) | Fingerprint filtering followed by exact comparison when required. |

Graph updates prepare every changed frame vertex before installing frame relationships. That lifecycle order ensures every referenced vertex exists; it does not impose an execution order on Photonic states. Gate arrival order remains arbitrary. Candidate filtering is only a prefilter: token multiplicity, resource identity, and full gate constraints remain the responsibility of matching and execution.

Hash aggregation intentionally forgets neighbor order, while retaining multiplicity through sum, squared sum, and count. Full and incremental refinement use the same contribution formula. Hash equality remains a filter, never a proof of semantic equality.

## Structural maintenance

`language/structure.rs` maintains world, frame, and resource vertices across structural comparisons. Immutable world identity and resource identity identify retained vertices; frame changes replace the relevant relationships. These identifiers are internal index keys and never contribute to the fingerprint. Vertex labels and incidence types match `language/incidence.rs`.

`language/propagation.rs` maintains four rounds of neighborhood refinement. Each vertex retains the sum, squared sum, and count of its neighbors' contributions at each round. Edge insertion and deletion adjust those accumulators. A changed color updates dependent contributions at the following round. If more than half the live vertex count is queued, the round switches to a full pass to avoid expensive dense propagation.

The resulting fingerprint is identical to full four-round recomputation. Eight-round filtering and exact canonicalization still handle remaining collisions. Repeated membership, distinct equal-valued resources, captures, lexical relationships, and the distinguished root remain represented. A state containing repeated references to the same world allocation uses full recomputation, preserving multiple world occurrences.

Only one mutable refinement cache is retained by a path search. Historical fingerprints remain immutable scalar values. The cache participates in record accounting and is discarded before optional cache retention can stop a search at its record budget.

## Matching and synchronization

The world index records removed and inserted sites. Cached selections retain every input domain, including domains belonging to an impossible conjunction. A selection update retracts removed membership and checks inserted worlds against its frame, symbol, and capture constraints. It preserves the selection allocation when membership is unchanged, including safe reuse of a site for a different world. Token enumeration still reads the current world, so unchanged candidate membership does not imply unchanged token bindings.

Capture-free input patterns share compiled terms and selections across lexical owners. Rules still retain their own owner, output, and read dependency. Capture-dependent patterns keep separate owner keys. Cached entries are evicted when no longer requested, and updates across a missing generation fall back to fresh preparation.

The gate matcher stores partial bindings as parent-linked arena entries. Extending a binding shares its existing prefix; only a complete result becomes a flat slot vector. Distinct-world constraints, synchronization, and symmetry checks retain their existing meaning. Matching order is an internal search optimization. The candidate-order regression test ensures that a reordered domain uses its corresponding pattern, and both direct and exhaustive evaluators reach the expected result.

The runtime still reconstructs rule scheduling after a transition. Candidate membership persists across transitions; gate prefixes are shared within a search. This implementation does not claim full cross-transition join maintenance or the complexity guarantees of a complete worst-case-optimal join engine.

## State layout

A transition computes reachable frames once for reclamation, then retains its cell count, next resource identifier, and reachable frame list alongside its fingerprint. Application, fingerprint aggregation, and limit checks reuse that layout. Historical states and complete transition provenance remain inspectable. State collections still share immutable worlds and frames; history has not been replaced by lossy summaries or checkpoints.

## Selection and verification

The design borrows incremental view maintenance principles from [Differential Dataflow](https://github.com/TimelyDataflow/differential-dataflow) and [DBSP](https://docs.feldera.com/vldb23.pdf). Shared intermediate representation follows the direction of [MAVIS](https://github.com/pkumod/MAVIS) and factorized query processing. These are algorithm references, not new dependencies.

A per-frame activation index was implemented and measured, then removed: maintaining its additional symbol counts and constructing local request lists was slower on the benchmark matrix. The selected implementation also avoids eagerly materializing every complete match.

Regression coverage compares incremental fingerprints with independent full recomputation across forward and reverse state sequences, renaming, repeated world references, and 1,024 generated mutations. Matching coverage includes candidate retraction, site reuse, capture reassignment, owner sharing, input multiplicity, unequal candidate cardinality, and independently enumerated multiway gate results. Existing exhaustive reference, provenance, pause/resume, budget, browser, and program tests remain enabled.

## Measurement

Measurements compare commit `b8010d2` with this implementation on the same Apple M5 Max running macOS 26.6.2. Every program runs through `bazel run -c opt`, with one warmup and three measured samples. Parsing, compilation, initialization, reporting, and release are outside the execution timer. Runs are sequential, without concurrent builds or tests. Raw samples are in [incremental.json](incremental.json).

| Program | Before | After | Speedup |
| --- | ---: | ---: | ---: |
| Six factors of two | 0.0809 s | 0.0648 s | 1.25× |
| Ten factors of two | 0.2308 s | 0.1850 s | 1.25× |
| Twenty factors of two | 1.3746 s | 1.0127 s | 1.36× |
| Thirty factors of two | 4.3365 s | 3.0387 s | 1.43× |
| Nested ternary expression | 0.0641 s | 0.0526 s | 1.22× |
| Ten-digit decimal addition | 0.2803 s | 0.1909 s | 1.47× |
| Five-digit decimal multiplication | 0.2901 s | 0.2185 s | 1.33× |
| Preparation reuse workload | 0.0162 s | 0.0148 s | 1.09× |

Every program reaches its expected result with the same successful transition count. Thirty factors still execute 151,476 transitions. This work improves implementation cost; it does not remove the separate browser state limit or establish an exponential or thousand-fold speedup.

Reproduce representative cases with:

```sh
bazel run -c opt //benchmark:expression -- '2*2*2*2*2*2' 2101 --sample 3
bazel run -c opt //benchmark:arithmetic -- 1234567890 add 9876543210 --radix 10 --sample 3
bazel run -c opt //benchmark:arithmetic -- 12345 multiply 67890 --radix 10 --sample 3
bazel run -c opt //benchmark:retention -- --width 200 --length 1000 --sample 3
```

## Refactoring verification

The module separation above was checked against `a7a5f62` with the same native Bazel harness. All 108 test targets, including browser conformance, pass; Rust formatting and lint checks pass. Successful transition counts, work counts, and every reported search statistic agree across the measured cases.

| Program | Before | After |
| --- | ---: | ---: |
| Six factors of two | 0.0667 s | 0.0650 s |
| Thirty factors of two | 2.9823 s | 3.0256 s |
| Ten-digit decimal addition | 0.1885 s | 0.1906 s |
| Preparation reuse workload | 0.01333 s | 0.01331 s |

Values are median execution times. The first three rows use three measured samples after one warmup. The short reuse workload initially measured 0.0131 s versus 0.0152 s; repeating both revisions with fifteen samples produced the values shown. These measurements support comparable performance for this cleanup, rather than a speedup claim. Initial and repeated samples are retained in [refactoring.json](refactoring.json).
