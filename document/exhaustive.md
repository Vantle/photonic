# Immutable preparation across exhaustive queries

This stage follows [capacity rejection](residual.md), with baseline `99c17a5`. It extends immutable matching preparation into the exhaustive evaluator. It does not share mutable gates or collapse proof consumers, and does not yet share joined multi-input results across distinct exhaustive queries.

## Ownership

`preparation/cache.rs` owns the bounded cache of immutable preparations. Keys contain a weak immutable pattern identity, a weak world identity, and an explicit context discriminator. The weak control blocks prevent pointer reuse from matching a different object. A changed copy-on-write world therefore cannot reuse preparation from its previous identity. Missing entries are prepared outside the mutex; publication rechecks the key so concurrent equivalent requests can use the same immutable value.

The direct `preparation::Store` supplies compiled symbol patterns and their relevant lexical owner. `selection::Store` supplies fully captured term patterns. These are different concrete cache instantiations, so direct and exhaustive identities cannot be confused. Each returned `particle::Match` owns its private combination selection and reset state.

The exhaustive store interns exact, normalized term vectors before preparing individual worlds. Captured executable rules remain distinct. Equal labels with different captures do not share a compiled pattern. Hash lookup discovers a candidate; exact vector equality establishes identity.

The first consumer uses private compilation. A later consumer may use sharing; cache storage initializes only when needed. The outer store stays compact while unused. Compiled fragments and particle preparations share one 65,536-record budget. Pressure prunes unused compiled identities and expired preparation keys; failed admission always falls back to exact private matching. Particle preparation remains limited to the existing cost-admission region of wider patterns and worlds, not to a restricted language feature set.

`runtime::table::Table` owns the optional exhaustive store. Individual searches retain their original candidate domains, gate state and logical delivery position. Consumers retain source inference, flow projection, executable reads and competing evidence. The runtime counts shared retained records and evicts this recomputable cache before a record limit suspends execution. Active cursors remain valid after eviction through their immutable references.

## Validation boundary

The tests compare private and shared searches under small and saturated capacities, independent interleaving, reset, copy-on-write replacement, capture changes and eviction. Concurrent requests exercise immutable publication. End-to-end exhaustive tests compare every budgeted snapshot, including proof evidence and work, both under ordinary chunking and record limits that force eviction. Those comparisons exclude the intentional additional shared-cache record and peak footprint; the existing nineteen exhaustive fixtures are also checked for exact record and peak equality.

The native benchmark constructs distinct two-input exhaustive queries with a shared wide particle. The baseline harness calls its original `Search::new`; the new harness calls shared search with either a retained store or a new store per query. World construction, indexing, query patterns, polling, work and binding checks are identical. The audit records both harness hashes and the baseline adapter rather than representing the old runtime as implementing the new API.

Timing uses `bazel run -c opt`, alternating paired runs and AC power. Each case checks `kern.sleeptime` before and after execution. Interrupted cases are discarded and retried. Dense and small worlds have both repeated-query and cold-query controls, alongside the full protected direct and exhaustive matrix. The [raw audit](exhaustive.json) records all samples, source hashes, the full baseline adapter and validation. The final accepted run contains no sleep interruption.

## Remaining boundary

This is immutable preparation sharing beneath exhaustive queries. Multi-input gate prefixes, source projection and parameterized proof construction remain separate work. The next proposed CPU stage is exact batching of cached waiting spans, with unchanged budget prefixes, memory accounting and synchronized firings. GPU execution remains conditional on a measured bulk workload that can amortize submission and transfer.

## Results

Three alternating rounds of nine parameterized samples produced these pooled medians:

| Workload | Before | After | Speedup |
| --- | ---: | ---: | ---: |
| 256 exhaustive queries, dense shared world | 9.527 ms | 0.464 ms | 20.52× |
| Dense world, new store for each query | 9.597 ms | 9.543 ms | 1.01× |
| Small world, repeated queries | 0.642 ms | 0.383 ms | 1.68× |
| Small world, new store for each query | 0.645 ms | 0.642 ms | 1.00× |
| Protected prefix workload | 3.313 ms | 3.332 ms | 0.99× |
| Six factors of two | 65.136 ms | 64.492 ms | 1.01× |
| Ten-digit addition | 196.931 ms | 195.612 ms | 1.01× |
| Five-digit multiplication | 219.598 ms | 216.737 ms | 1.01× |

The cache improves repeated exhaustive preparation, not every proof query or arithmetic program. All protected controls passed the declared five-percent gate, with ten percent allowed for submillisecond controls. Initial eager-cache overhead on cold small queries was removed through lazy storage and first-consumer bypass. A protected prefix regression was investigated with the new native [`traversal`](https://github.com/Vantle/photonic/blob/0daf296e1d126675ed445bfb7ed6674a379b1532/benchmark/prefix.rs) phase profiler, removed after `0daf296`, and resolved by retaining inline dispatch/accounting boundaries and separating binding reconstruction from the common replay path.

Final verification passed all 106 optimized Bazel test targets, including native/WASM conformance, all 189 debug kernel tests, formatting and diff checks. Existing serialized subscription observations and the nineteen exhaustive fixture observations retain their required equality. Additional wide proof tests preserve every step's evidence and work under four-worker execution and record limits that force cache eviction.
