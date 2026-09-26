# Runtime optimization status

The native evaluator now combines shared input plans, dependency-driven dispatch, persistent world indexes, lazy joins, bounded binding replay, compiled rewrite recipes, persistent state storage, and incremental structural filtering. These are production implementations. They do not establish a universally optimal evaluator, and several broader research recommendations remain incomplete.

Photonic remains unordered. A firing requires a complete synchronized binding, including resource multiplicity, capture ownership, and executable-rule read dependencies. No arithmetic operation is recognized or replaced by host arithmetic. The language, frontend behavior, browser limits, and inspectable proof contents are unchanged.

## Recommendation audit

| Recommendation | Status and boundary |
| --- | --- |
| Represent evaluation as a typed incidence graph | Implemented in `incidence.rs`, with distinct containment, resource, capture, parent, and lexical relationships. Exact canonicalization resolves fingerprint collisions. |
| Share identical matching plans and schedule them incrementally | Implemented in `catalog.rs` and `dispatch/`. Symbol dependencies are compiled once, and sparse changes use dependency lookups. Small or dense queries use direct scans. |
| Keep candidate domains and particle preparation across rewrites | Implemented in `joining.rs`. Removed sites invalidate their particle cache even when their numeric slot is immediately reused. |
| Avoid materializing join products eagerly | Implemented with cardinality-ordered lazy traversal, distinct-world feasibility checks, and existing symmetry restrictions. |
| Reuse partial and complete joins across unchanged transitions | Implemented in `replay.rs` as a bounded stream cache. It preserves both binding results and pending steps, including interrupted enumeration. |
| Maintain every surviving join product after a domain changes | Not implemented. Relevant domain changes invalidate the replay stream while retaining unaffected candidate preparation. Unrestricted materialization could itself require combinatorial storage. |
| Use a general worst-case-optimal relational join engine | Not implemented. The current literal matcher uses candidate domains, exact token combinations, and an all-different world constraint. Adaptive invalidation is not a claim to implement Free Join or a general relational join engine. |
| Apply explicit state changes and share history storage | Implemented in `change.rs`, `sequence.rs`, and `path.rs`. Historical states and transition provenance remain available. |
| Compile generic output construction | Implemented in `recipe.rs` and `rewrite.rs`. This is recipe compilation, not a JIT or fused multi-transition execution. |
| Incrementally maintain structural fingerprints and reachability | Partially implemented. Unchanged frame structure reuses hashes and reachability roots. Changed frame topology and roots retain exact full-traversal fallbacks. |
| Reuse backward proof matching | Implemented per immutable target, frame, and pattern in the exhaustive runtime. Its separate matcher and proof projection remain active; the direct path dispatcher does not replace backward inference. |
| Reuse independent rewrite outputs across different contexts | Not implemented. Binding replay reuses matching, not state construction or proof projection. |
| Replace history enumeration with causal unfolding or partial-order reduction | Not implemented. Preserving reachability alone is insufficient: observable histories, competing evidence, captures, and backward proof dependencies must also be preserved. |
| Achieve orders-of-magnitude improvement on arithmetic | Not achieved. The large improvements are specific to measured matching workloads. Arithmetic still performs its original Photonic transitions. |

The remaining rows are explicit limits, not hidden implementation switches. No execution is discarded using an unproved independence assumption. Structural rule construction remains the separate language proposal described in [dynamic.md](dynamic.md); these implementation changes neither introduce that syntax nor impose a new semantic limit on it.

## Dispatch and query execution

The dispatcher maintains counts per frame and, for larger networks, an index of entries with viable candidate assignments. It avoids repeatedly resetting unrelated impossible matchers. Retained entry storage is updated when entries change, so checking the record budget no longer scans every matcher.

Dependency selection is adaptive. Looking up a common symbol can visit more global plans than a small frame contains. Small frames therefore scan their local entries; larger frames compare local entry count with dependency fanout. Consumer reconciliation makes the same choice between direct frame scans and selected input ranges. Reuse counters are maintained from frame counts without visiting every unchanged entry.

Posting lists preserve insertion order. Their stable sites map to monotonically ordered physical positions, including after tombstone compaction. Intersections use the shortest posting, skip testing that posting against itself, and use binary search in larger other postings. Small postings retain linear search. Single-term queries directly enumerate their posting without allocating an intermediate posting list.

Previously, even a single-term lookup performed a linear membership search for every member of its own posting, giving quadratic membership work. The direct path removes that work. For multiple terms, each large posting membership check becomes logarithmic. Translating returned sites to current world positions still uses the order-statistic index; posting deletion can still require a linear pass.

Compiled plans store unique symbol dependencies independently of pattern multiplicity. Particle preparation also discovers the candidate domain for an identical term once, while retaining every required occurrence in the exact token combination matcher. These optimizations preserve the distinction between one resource and several equal-valued resources.

## Bounded join replay

A matcher starts without a replay allocation. After it has been visited and survives a reset, it can record its result stream. Consecutive pending steps are stored as a count. Bindings store stable sites, input positions, and exact resource identifiers; replay resolves sites to the current world positions.

The underlying join remains at the recorded stream's frontier. A subsequent traversal replays the prefix and then continues that suspended join. This works when evaluation stops after only part of a match, rather than requiring the whole Cartesian product to finish first. Relevant insertions, removals, or capture changes discard the stream through the existing dependency invalidation path.

The cache is capped at 4,096 accounting units per matcher. Exceeding the cap discards the cache and continues streaming from the current join position. It does not truncate bindings, limit gate width, or change program results. Cache storage participates in the existing record budget. Resource accounting measures the runtime's logical records, not exact allocator bytes.

## Module and API boundaries

| Responsibility | Module |
| --- | --- |
| Entry ownership, lifecycle, and scheduling | `dispatch.rs` |
| Dependency activation and adaptive invalidation | `dispatch/dependency.rs` |
| Visible rule and executable occurrence discovery | `dispatch/consumer.rs` |
| Binding delivery to rule consumers | `dispatch/entry.rs` |
| Named matching identity and ordered lookup ranges | `dispatch/key.rs` |
| Exact lazy join and bounded result replay | `joining.rs`, `replay.rs` |
| Exhaustive scheduling and proof state | `runtime.rs` |
| Exhaustive matching, subscriptions, and projection | `runtime/matching.rs` |
| Proof application and canonicalization completion | `runtime/application.rs` |
| Public proof reporting | `runtime/report.rs` |

Matching keys explicitly name frame, input, and owner. Read dependencies explicitly name site and resource. Exhaustive query requests explicitly name target, frame, and pattern. A join retains its frame at construction, so advancing it cannot accidentally supply a different frame. Entry cursors, selected bindings, and join state are private. Polling consistently returns `Poll<Option<T>>` for progress, delivery, or exhaustion.

The exhaustive matcher remains algorithmically separate and continues to serve as a semantic reference. All new Rust source files have explicit Bazel entries. The changes add no dependency, frontend compatibility layer, or build-time system prerequisite.

## Verification

All 108 Bazel test targets pass, including native programs, browser conformance, build-tool checks, proof snapshots, and pause/resume tests. Formatting and lint checks pass. Native and WebAssembly builds are verified on macOS; Linux and Windows execution has not been tested on this host.

Additional regression coverage includes:

- Exact polling and binding sequences during interrupted, repeated join traversal.
- Cache overflow followed by continued complete enumeration.
- 512 token-matching cases compared with independent exhaustive enumeration, including duplicate resource occurrences and capture-specific terms.
- 1,024 posting-index mutations compared with a direct scan, covering captures, empty patterns, site reuse, and compaction.
- 256 dispatcher mutations compared with fresh networks, preserving duplicate deliveries as well as distinct results.
- Capture changes, lexical changes, executable read dependencies, and repeated resets.
- Readiness-index promotion, frame growth and removal, and retained-storage accounting after every dispatch step.

The full suite required no changes to saved frontend records.

## Measurement

These measurements compare `c0a597c` with the implementation accompanying this document on an Apple M5 Max running macOS 26.6.2. Both revisions use identical native Bazel benchmark harnesses and `-c opt`. Cases run sequentially as before/after pairs, alternating which revision runs first, after tests and builds finish. Every benchmark case receives at least 100 ms of warmup on both revisions. Shorter warmups showed startup drift on small cases and were excluded from these final tables. Raw samples, arguments, work counters, initialization times, and a digest of the candidate Rust and Bazel source files are in [optimization.json](optimization.json).

The execution timer includes establishing the result. Parsing, initialization, reporting, and release are outside it. The final column also includes runtime initialization. Values below 1× are slowdowns. Successful transition counts and logical work counts agree between revisions in every path benchmark.

| Workload | Before (ms) | After (ms) | Execution speedup | Including setup |
| --- | ---: | ---: | ---: | ---: |
| Six factors of two | 66.776 | 70.865 | 0.94× | 0.94× |
| Ten-digit decimal addition | 192.519 | 200.483 | 0.96× | 0.96× |
| Five-digit decimal multiplication | 226.469 | 224.723 | 1.01× | 1.01× |
| 16 untouched worlds | 1.116 | 1.258 | 0.89× | 0.93× |
| 10,000 untouched worlds | 16.479 | 16.424 | 1.00× | 0.99× |
| 512 distinct unchanged matchers | 16.613 | 2.548 | 6.52× | 4.36× |
| 4,096 distinct unchanged matchers | 126.626 | 8.015 | 15.80× | 7.35× |
| 1,024 repeated tokens | 10.212 | 1.887 | 5.41× | 3.96× |
| 4,096 repeated tokens | 62.090 | 3.916 | 15.86× | 10.53× |
| Ten factors of two | 188.872 | 199.299 | 0.95× | 0.95× |
| Thirty factors of two | 3,102.676 | 3,326.183 | 0.93× | 0.93× |

The distinct-matcher workload retains one `IdleN` world for each `[IdleN,IdleN]` input while an independent world advances through 1,000 ordinary rules. The repeated-token workload retains a particle with one fewer `A` resource than an unchanged pattern requires, alongside the same transition chain. These isolate generic scheduling and multiplicity costs. They are not arithmetic throughput measurements.

The posting benchmark isolates candidate lookup against an already-built index. It checks the result, then collects seven samples of ten lookups each. The following medians are per lookup; they are not end-to-end program speedups.

| Worlds | Query terms | Before (µs) | After (µs) | Lookup speedup |
| ---: | ---: | ---: | ---: | ---: |
| 128 | 1 | 0.912 | 0.338 | 2.70× |
| 128 | 2 | 0.629 | 0.583 | 1.08× |
| 1,024 | 1 | 45.746 | 2.763 | 16.56× |
| 1,024 | 2 | 33.188 | 5.075 | 6.54× |
| 16,384 | 1 | 9,124.846 | 56.950 | 160.23× |
| 16,384 | 2 | 6,971.742 | 120.921 | 57.66× |

The exhaustive runtime benchmark covers 19 closed reference cases, with 25 measured samples per case. It includes initialization, execution, snapshot construction, and runtime release. The median of the per-case speedup ratios is 0.99×, ranging from 0.91× to 1.05×. State, event, and logical work counts agree. The module cleanup and removal of repeated frame inspection do not establish an overall proof-search speedup on these small cases.

Addition and the expression cases are about 4–7% slower than the previous commit in this run; multiplication remains near baseline. Small workloads can also pay proportionally more for the additional bookkeeping; the 16-world case is reported explicitly above. Additional matching state and bookkeeping have a cost when reuse is limited. Thirty factors still require 151,476 transitions. These results support large improvements on the measured matching bottlenecks, not a claim of a universal thousand-fold evaluator improvement.

Reproduce the benchmarks and checks with:

```sh
bazel run -c opt //benchmark:expression -- '2*2*2*2*2*2' 2101 --sample 7
bazel run -c opt //benchmark:arithmetic -- 1234567890 add 9876543210 --radix 10 --sample 7
bazel run -c opt //benchmark:storage -- --width 4096 --length 1000 --rule --sample 7
bazel run -c opt //benchmark:runtime
bazel test -c opt --nocache_test_results //... //toolchain/browser:check
```

These benchmarks were removed after `0daf296` and still run at that commit:

- [`replay`](https://github.com/Vantle/photonic/blob/0daf296e1d126675ed445bfb7ed6674a379b1532/benchmark/replay.rs) with `--width 4096 --length 1000 --sample 7`
- [`posting`](https://github.com/Vantle/photonic/blob/0daf296e1d126675ed445bfb7ed6674a379b1532/benchmark/posting.rs)

The earlier [persistent-state measurements](incremental.md) remain a separate comparison against their original baseline; their speedup ratios should not be multiplied into this table without a new paired measurement.
