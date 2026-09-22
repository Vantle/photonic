# Canonical reporting maintenance

Baseline: `dc7b249`. This change implements the first reporting optimization selected by the [lifecycle audit](lifecycle.md). It changes partition refinement and temporary graph construction, preserving the existing rules, canonical order, resource mappings, search steps and report format. Performance acceptance targets the current Apple M5 Max; the implementation uses portable safe Rust and adds no dependency.

## Representation and invariants

Refinement still partitions vertices by their existing color and their sorted outgoing edge-kind/neighbor-color signatures. Colors are updated together after a round. The new implementation records each partition's end at its start position in the vertex ordering. A split inserts new boundaries within that interval; other intervals and their start positions remain stable. These start positions supply ordered temporary colors. Final traversal assigns the same dense ordinal colors as before.

Previously each round scanned every partition, searched its vertices for an affected flag and copied all partition ranges into a second vector. The replacement queues only nonsingleton partitions whose outgoing signatures can have changed. Splitting a group records changed vertex colors. After all colors are installed, incoming edges identify the groups that must be scheduled for the next round. A flag per possible group start suppresses duplicate scheduling. Singleton partitions cannot split and are never queued.

This preserves synchronous refinement: processing order within a round does not affect any signature's input colors. A group without an edge to a changed color retains equal signatures within each existing class. A newly split group is scheduled again if one of its neighbors changes. The first subgroup retains the old start color; only vertices whose actual temporary color changes propagate work. Empty graphs and initially unique colors return without building the remaining refinement workspace.

Workspace remains O(V + E), including graph storage, signatures, boundaries, queue, colors and reverse dependencies. The change removes repeated scans and copies of unaffected partitions. It does not make general refinement linear: a large affected group can still be sorted repeatedly. No state is shared across refinement calls, so there is no new cache, eviction or invalidation policy.

Graph construction now accepts a cloneable iterator over edges. It counts degrees in one pass and fills contiguous adjacency storage in a second. Every production caller supplies an immutable, repeatable iterator. Incoming dependencies can therefore be built directly from the original adjacency traversal, without first allocating a separate reversed-edge list. Vertex order, edge kind, multiplicity and per-source edge order are preserved. This also removes capacity-growth allocation from that temporary list.

## Verification

The existing independent refinement oracle repeatedly constructs complete signatures and orders their distinct values. Tests compare exact color numbers, not merely partition equivalence. Coverage includes every directed three-vertex graph under binary initial labels, deterministic larger graphs, duplicate edges, propagation chains, empty graphs and singleton graphs.

New scheduling cases add noncontiguous and reversed unique colors, repeated edges, self-loops, shared anchors, propagation around a cycle and reversed vertex/adjacency order through 128 vertices. The graph test also compares constructed adjacency lists exactly with their source lists. This exercises the changed iterator interface and edge multiplicity independently of the refinement result.

All 109 optimized Bazel test targets pass: 105 execute freshly and four unchanged targets use cached results. Coverage includes native/WebAssembly conformance, kernel semantics, suspension/resumption and the historical differential evaluator. Formatting and build-integrated lint checks pass.

## Measurement

The final audit uses separately retained baseline executables and final candidate executables, optimized through Bazel. Three rounds alternate baseline/candidate order. Lifecycle cases record one cold evaluation and five subsequent fresh-instance samples per process. Compact execution and inspection use nine samples; the existing symmetry benchmark uses 25 per case. Timing runs are serial after builds and tests complete. Allocation diagnostics run separately and are not used as latency evidence.

A requested 128-stage inspection case exceeded that benchmark's fixed 40,000-work budget on the baseline. The protected comparison therefore uses its established 32-stage case; this configuration failure is recorded in the raw artifact. The direct 128-step lifecycle case remains included separately.

The [raw audit](canonical.json) preserves every sample, source/executable hash, baseline revision, power state, preliminary experiment and exact-report comparison. All state/event/work counts, canonical search steps and report fingerprints agree across the compared runs. Protected gates permit at most 5% regression, or 10% below one millisecond. The complete lifecycle, compact execution, inspection and warmed symmetry controls pass.

| Workload | Baseline | Candidate | Baseline / candidate |
| --- | ---: | ---: | ---: |
| Six factors, full reporting | 613.230 ms | 553.027 ms | 1.11× |
| Six factors, complete lifecycle | 807.880 ms | 746.859 ms | 1.08× |
| Three factors, complete lifecycle | 151.030 ms | 144.644 ms | 1.04× |
| Nested expression, complete lifecycle | 232.456 ms | 221.650 ms | 1.05× |
| Compact six-factor execution | 64.206 ms | 64.015 ms | 1.00× |
| Ten-digit decimal addition | 198.496 ms | 199.099 ms | 1.00× |
| Five-digit decimal multiplication | 219.395 ms | 220.844 ms | 0.99× |
| Incremental inspection | 5.586 ms | 5.477 ms | 1.02× |

Other lifecycle controls cover addition, a direct chain, exhaustive diamond exploration, insufficient token/coherence capacity and 512 distinct or uniform worlds. The tiny diamond case increases from 40.25 to 42.96 microseconds, within its 10% tolerance; this is not a universal speedup.

The original symmetry benchmark took only 25 immediate samples per case. Initial comparisons showed unstable small-case regressions, including the first two-world case; fifteen additional alternating comparisons did not resolve that first-case signal. The benchmark now warms each case for 100 ms before taking the same 25 samples, matching the existing timing practice in other benchmarks. The identical warm-up change was applied to an isolated baseline checkout. Seven alternating warmed comparisons put the two-world private case at 1.459 microseconds on both sides; all twelve warmed symmetry controls pass their original tolerance. Unwarmed results remain in the artifact. These measurements establish steady-state behavior, not identical cold-start performance under every processor power state.

For the six-factor lifecycle, median requested allocation traffic falls from 2,474,641,477 to 2,218,843,445 bytes, a 10.3% reduction. Reporting removes 10,101 allocations and 160,926 reallocations in the sampled run. Peak live requested storage remains 515,832,240 bytes: retained canonical states, materialized report nodes and the output buffer still dominate. Every allocation diagnostic returns to zero additional retained bytes on release. Instrumented timings are not used for speed claims.

Sixteen direct/exhaustive command outputs compare byte-for-byte against the baseline, including a 69,596,556-byte complete precedence report, three paused product reports and twelve paused/completed diamond or captured-rule reports. Comparisons include the complete serialized fields, without filtering work or storage accounting. Their argument lists, byte lengths and digests are recorded after direct byte comparison.

The result is a measured reporting improvement with essentially unchanged arithmetic execution. Remaining reporting cost includes incidence construction, canonical state renaming, retained canonical forms and node/string materialization. Any next change still needs its own complete-program and memory gate; this result does not authorize changing rules or dropping history.

## Reproduction

```sh
bazel test -c opt //...
bazel run -c opt //:format -- --check
bazel run -c opt //benchmark:lifecycle -- expression '2*2*2*2*2*2' 2101 --sample 5
bazel run -c opt //benchmark:allocation -- expression '2*2*2*2*2*2' 2101 --sample 3
bazel run -c opt //benchmark:expression -- '2*2*2*2*2*2' 2101 --sample 9
bazel run -c opt //benchmark:symmetry
```
