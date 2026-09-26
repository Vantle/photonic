# Shared joins and proof reuse

This audit follows checkpoint `888f69e` and compares the final implementation with `20984f1`. The exact source manifest, paired native measurements, and validation commands are in [sharing.json](sharing.json). The [prefix audit](prefix.md) remains the historical report for the checkpoint.

The implementation shares completed joined prefixes between different input plans, reuses canonical capture environments and identity flow compositions, and removes repeated work from matching and dispatch. Frontend syntax and evaluation semantics are unchanged. No arithmetic operation receives special treatment.

## Ownership

| Component | Responsibility |
| --- | --- |
| `joining.rs` | Named preparation request, query ordering, admission and invalidation |
| `joining/space.rs` | Candidate membership and retained particle preparation |
| `joining/cursor.rs` | Exact streaming enumeration and resource distinctness |
| `joining/product.rs` | Synchronize a reusable prefix with its remaining input |
| `joining/stream.rs` | Admission, recording, replay, fallback and restoration |
| `joining/trace.rs` | Budgeted transcript with stable site and token selections |
| `joining/playback.rs` | Independent delivery cursor and consumer-position projection |
| `joining/key.rs`, `node.rs`, `store.rs` | Exact shared-prefix identity, snapshot validation and bounded discovery |
| `runtime/environment.rs` | Weak index of canonical capture environments |
| `runtime/composition.rs` | Flow composition and its proof-support obligation |

The dispatch network owns the shared join registry. Queries own their mutable enumerators and playback positions. A completed transcript can have multiple readers; its content is immutable. The registry holds a weak reference to the transcript, so an unused registry entry cannot retain its binding data.

`Join::planned` and `Search::planned` now accept a named request. Reset receives the current index explicitly. Cache internals remain private; production consumers cannot mutate a transcript or another query's cursor.

## Preservation argument

A shared prefix key contains its frame and exact particle patterns in traversal order, including captured rule identities. Different input positions can share the same prefix because stored selections contain only stable sites and token indices. Playback restores each consumer's own input positions. Particle ordering is not normalized beyond this positional projection, and incomplete recordings are private.

A cross-query lookup must also match an index snapshot token by allocation identity. The token is created lazily, replaced on every index update, and kept alive by the registry version. Equal state values in separate indexes cannot accidentally share recycled site assignments, and holding the token prevents address reuse from validating an obsolete entry. Hash equality alone never establishes a hit.

Within one query, the existing invalidation rule still applies: a prefix survives only when its ordered input positions and candidate memberships survive. A valid local transcript can then be published for the new snapshot. Changing a prefix input discards its traversal before reuse. Suffix extension independently enforces world distinctness, duplicate-input symmetry, and exact resource multiplicity.

Recorded waits remain logical waits. Playback keeps the remainder of the current waiting run in its own cursor, avoiding repeated transcript lookups while returning exactly one `Poll` per original step. Eviction restores the underlying enumerator to the same logical position. No scheduler rounds, memory checks, or supported pause boundaries are skipped.

Duplicate-input symmetry now compares live index positions directly. Position compaction and insertion preserve the relative order of surviving worlds, so this comparison equals comparing their computed world ordinals. This removes two ordinal calculations from each comparison. It is an internal enumeration optimization; Photonic state remains unordered, and synchronization still establishes firings.

Proof reuse has separate validity conditions. Canonical environments are indexed by immutable target-state identity and capture frame. The index holds weak references to values already owned by normalization identities, and cannot keep an environment graph alive. Flow reuse recognizes the identity view created when a state is interned; equality of source and target alone would be insufficient because cycles can have nonidentity provenance. Composing that known identity view with an event returns the event's immutable flow. Every composition task still records its original evidence clause.

## Bounds and fallback

The shared registry and transcript allocations use the existing global 65,536-unit factor budget. A transcript retains at most 4,096 logical units and records at most 65,536 logical steps. Registry keys consume the same budget; inactive registrations are pruned under pressure. Admission failure, recording saturation and eviction resume exact streaming enumeration. These limits constrain optional reuse, not the programs or answers the runtime accepts.

Query accounting conservatively includes each query's logical transcript view, while the allocation budget charges the shared transcript once. Registry metadata is counted separately by the dispatch network. Retained logical counts are not allocator peak bytes.

The environment index holds at most 4,096 weak entries, pruning expired entries before admission. A full index computes the environment normally. Its entries index already owned normalization values, as the normalization identity index does; they do not add proof records or retain additional state graphs. Their bounded metadata allocation is separate from the runtime's logical proof-record count.

## Measurements

Three paired rounds used optimized native Bazel binaries on the same AC-powered Apple M5 Max, with warmup and alternating revision order. The tables show medians pooled across the available execution samples, in milliseconds. All compared work and binding/event counts agree. A ratio above 1 means faster execution.

| Component workload | Before ms | After ms | Speedup |
| --- | ---: | ---: | ---: |
| Repeated join, width 4 | 0.0701 | 0.0355 | 1.97× |
| Changing prefix, width 4 | 0.0973 | 0.1006 | 0.97× |
| Repeated join, width 8 | 1.7265 | 0.3982 | 4.34× |
| Changing prefix, width 8 | 1.7917 | 1.4671 | 1.22× |
| Repeated join, width 12 | 42.0928 | 9.0559 | 4.65× |
| Changing prefix, width 12 | 42.3620 | 32.7690 | 1.29× |
| 32 plans, private registry, width 4 | 0.0494 | 0.0521 | 0.95× |
| 32 plans, shared registry, width 4 | 0.0499 | 0.0066 | 7.58× |
| 32 plans, private registry, width 8 | 0.9007 | 0.8276 | 1.09× |
| 32 plans, shared registry, width 8 | 0.9097 | 0.1531 | 5.94× |
| 32 plans, private registry, width 12 | 20.1159 | 16.7222 | 1.20× |
| 32 plans, shared registry, width 12 | 20.0875 | 3.6994 | 5.43× |

| End-to-end workload | Before ms | After ms | Speedup |
| --- | ---: | ---: | ---: |
| Synchronized prefix: width 8, delay 32 | 7.0346 | 4.4161 | 1.59× |
| Synchronized prefix: width 12, delay 128 | 224.1828 | 78.0171 | 2.87× |
| Prefix changes every transition | 7.0754 | 4.8274 | 1.47× |
| Six factors of two | 68.8427 | 66.1228 | 1.04× |
| 1234567890 + 9876543210, decimal | 198.2736 | 198.5077 | 1.00× |
| 12345 × 67890, decimal | 222.2662 | 221.0513 | 1.01× |
| Leaf factor: width 256, 1000 changes | 3.5217 | 3.4631 | 1.02× |
| Domain: 4096 candidates, 1000 changes | 12.0317 | 12.0682 | 1.00× |

For the 19 exhaustive fixtures, the median per-case speedup is 1.019×, with per-case ratios from 0.977× to 1.067×. These use the three reported medians of 25 samples per case, rather than unavailable individual runtime samples. State, event, work, record and peak counts are identical.

The small changing-prefix control still costs 97.29 → 100.58 microseconds across 64 updates (3.4% overhead). The small private-registry control and some individual proof fixtures also have small costs. These remain measured tradeoffs; this audit does not claim that every workload became faster. The larger churn, shared-prefix and synchronized-program workloads improve substantially, while arithmetic and domain controls remain close to baseline.

At width 12, the current shared-registry harness retains 8,141 logical units after the timed pass versus 20,076 with private registries. This is a final retained-structure observation, not a measurement of allocator peak bytes.

Across one run of each proof fixture, instrumented composition calls change from 356 to 209; logical runtime work is unchanged.

Across one run of each proof fixture, instrumented canonicalization calls change from 488 to 470; logical runtime work is unchanged.

`joining` rotates 64 inputs and drains the query after each update. The changing variants mutate the prefix itself. `sharing` uses 32 different whole-input plans with one common prefix: it prepares the index, warms a producer, then times one pass through the remaining 31 consumers. Its private mode gives each query its own registry, providing a no-sharing comparison with the same implementation. Setup and producer warming are outside that benchmark's execution interval; retained counts are sampled after the timed pass.

`prefix` is an end-to-end program with synchronized transitions. Arithmetic and expression timings are native path execution. `runtime` measures the exhaustive proof engine; `proof` runs the same fixtures with diagnostic instrumentation. Instrumented timings are not substituted for native timing results.

The accounting experiment that updated aggregate query totals on every cursor mutation was rejected after the broader benchmark exposed a regression. Final accounting keeps local counters and reads a fixed-size traversal structure. Replay and streaming paths were separated so cache admission checks do not burden an already recording or directly streaming query. Small replay helpers are inlined; the large enumerator remains separate.

## Validation

Both debug kernel tests and `bazel test -c opt --nocache_test_results //...` pass: 106 test targets, including 150 kernel tests, Rust formatting and Clippy, Bazel formatting checks, native program checks, and browser/WASM conformance. Existing semantic expectations were unchanged.

New tests check shared allocation identity across different consumer positions, duplicate inputs, relevant and irrelevant mutations, interleaved readers, eviction and restoration, capture isolation, separate index histories, snapshot replacement, weak-reference expiration, budget denial and metadata reclamation. Existing prefix tests compare every pending/binding result with fresh enumeration, including overflow and the reconstruction bound. Index tests also compare the new ordering predicate through 1,024 mutations and compactions. Proof tests independently compare identity composition and canonical environments with full recomputation.

Deep metaprogramming, generated rule occurrences, captured execution, competing histories, recursive support and suspension remain covered by the full suite. Runtime construction of previously uncompiled rule shapes remains the separate [dynamic language proposal](dynamic.md); these implementation changes add no restriction to it.

The validation establishes no known semantic regression in this coverage. Linux/Windows execution, browser timing and allocator peak bytes were not measured locally.

## Remaining algorithmic boundary

Completed prefixes can now be shared across different plans in the direct dispatch path. Partial recordings still belong to one query. A changed prefix is invalidated rather than maintained by retracting individual joined rows. Sharing arbitrary internal subplans and transferring this machinery into the exhaustive proof matcher require further implementation and equivalence work.

Proof reuse now covers canonical capture environments and identity compositions. General contextual rewrite templates and cross-composition reuse of nonidentity flows remain separate opportunities. Batching logical waiting ranges across scheduler rounds, causal-history reduction and additional parallel mutations remain conditional research work. They are not necessary consequences of using a hypergraph representation and are not claimed complete by this audit.

## Reproduction

```sh
bazel run -c opt //benchmark:prefix -- --width 8 --delay 32 --length 100 --sample 9
bazel run -c opt //benchmark:prefix -- --width 12 --delay 128 --length 100 --sample 9
bazel run -c opt //benchmark:prefix -- --width 8 --delay 32 --length 100 --changing --sample 9
bazel run -c opt //benchmark:expression -- '2*2*2*2*2*2' 2101 --sample 9
bazel run -c opt //benchmark:arithmetic -- 1234567890 add 9876543210 --radix 10 --sample 9
bazel run -c opt //benchmark:arithmetic -- 12345 multiply 67890 --radix 10 --sample 9
bazel run -c opt //benchmark:domain -- --width 4096 --length 1000 --sample 9
bazel run -c opt //benchmark:runtime
bazel test //language:test
bazel test -c opt --nocache_test_results //...
```

These benchmarks were removed after `0daf296` and still run at that commit:

- [`joining`](https://github.com/Vantle/photonic/blob/0daf296e1d126675ed445bfb7ed6674a379b1532/benchmark/joining.rs)
- [`sharing`](https://github.com/Vantle/photonic/blob/0daf296e1d126675ed445bfb7ed6674a379b1532/benchmark/sharing.rs)
- [`factor`](https://github.com/Vantle/photonic/blob/0daf296e1d126675ed445bfb7ed6674a379b1532/benchmark/factor.rs) with `--width 256 --length 1000 --sample 9`
- [`proof`](https://github.com/Vantle/photonic/blob/0daf296e1d126675ed445bfb7ed6674a379b1532/benchmark/runtime.rs)

The expression target uses ternary numerals; the arithmetic commands explicitly select decimal operands. Baseline reproduction uses an isolated `20984f1` worktree with the benchmark harnesses registered in explicit Bazel source lists. Internal benchmark adapters use its original `Budget`, positional preparation arguments and argument-free reset. They do not alter its runtime implementation. The JSON records hashes for both sets of harness files.
