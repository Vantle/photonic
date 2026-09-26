# Shared matching implementation

This implements the first production stages of the [roadmap](roadmap.md): deeper semantic coverage, phase measurement, matching and normalization ownership, shared immutable particle plans, conservative count summaries, and more selective maintenance. It does not implement the entire research roadmap. The comparison baseline is `03c5606`. The subsequent [incremental matching and reachability implementation](factorization.md) reports the next production changes separately.

## Matching and reuse

`pattern.rs` compiles a particle into distinct symbol requirements with every original input position retained. `catalog.rs` interns these fragments across different complete inputs. `plan.rs` binds a fragment to its capture owner when a concrete query needs it. Sharing code does not share executable occurrences, resource identities, or proof evidence.

Candidate filtering uses each distinct requirement once. Exact token matching still preserves every required occurrence. Posting entries additionally retain a token occurrence count per site and term. For wider patterns, a count below the required multiplicity establishes that the particle cannot match, avoiding token-search allocation. Counts are an upper bound on distinct resources when a token identity is repeated, so passing this check never establishes a match. Exact matching remains responsible for distinct identities and captures. Pattern width four is the current cutoff below which direct preparation avoids the extra summary lookup.

The count summary does not remove a world from the candidate domain or reorder enumeration. An impossible particle returns the same exhausted stream that exact preparation would produce. Its smaller retained representation can use fewer logical records. Record limits retain their existing meaning as implementation record accounting, rather than exact allocator bytes; browser limit values are unchanged.

Replay now survives an invalidation when inspection establishes that the actual candidate domains did not change. A removed and reinserted site invalidates its preparation even if its numeric slot is reused. If a domain changes, the full replay stream still resets, while preparation for surviving members remains available. This is not general maintenance of all surviving join products.

Posting insertion exploits its existing append order, avoiding a scan of the posting for each token. Deletion marks removed positions, filters each affected posting and reader list once, and then compacts positions before inserting new sites. This avoids repeated scans for repeated tokens or a batch of consumed worlds.

Dispatch updates scope membership only when an input becomes enabled or disabled. Consumer discovery intersects the smaller relevant input set with scope availability. These changes remove redundant maintenance without changing the consumer or delivery order.

## Ownership and construction

`runtime/table.rs` owns immutable-target matching queries, search storage, subscribers, delivery cursors, and matching accounting. It returns scheduling requests; the runtime retains agenda ownership and proof projection.

`runtime/normalization.rs` owns pending canonicalization, slot reuse, and the registry of pending and completed application identities. One registry replaces separate pending/completed lookups. Applications still retain their independent evidence, and unsupported cyclic evidence is not established by registry membership.

Rewriting now accepts a named request containing source, context, recipe, binding, and layout. Remainder construction uses resource membership lookup and a stable, deduplicated sequence instead of scanning the entire selected footprint for every token. Stable sorting preserves the existing first occurrence when a resource appears in several consumed worlds.

## Semantic gate

The new tests cover generated-rule execution, whole-rule matching against deep near-misses, and captured local execution at depths 1, 2, 3, 8, 16, and 32. Small exhaustive cases compare complete proof snapshots under uninterrupted and single-step execution. Direct and mutual recursive execution and repeated code production also remain bounded and cannot invent an absent result.

The depth-32 capture fixture requires more than 256 cells: its retained frame contents grow quadratically. Its test now verifies suspension exactly at the configured cell bound and resumption to the same semantic report with an explicit larger test budget. This changes no production limit. All five new metaprogramming tests also pass against the baseline revision.

Additional checks compare compiled preparation with naive token enumeration and the existing matcher, compare conservative filters and count summaries with direct scans, preserve exact polling sequences through replay, and exercise batch posting removal, compaction, shared resource identities, and slot reuse. Fragment sharing has an explicit test that equal code under incompatible capture owners cannot match through the shared plan.

All 106 Bazel test targets pass with uncached execution, including native programs and WebAssembly/browser conformance. The runtime target contains 122 passing unit tests. Default build aspects also pass Rust formatting, Clippy, and Buildifier checks. No saved frontend expectation has been changed. Validation was performed on macOS; other operating systems require CI execution.

## Profiling

Run an instrumented native expression with:

```sh
bazel run -c opt //benchmark:profile -- '2*2*2*2*2*2' 2101 --sample 3
```

The measurement build records phase call counts and elapsed time for matching, dispatch maintenance, index updates, rewriting, fingerprint updates, structural refinement, canonicalization, flow composition, and proof application. Counters are thread-local; this command uses sequential path execution. Initialization measurements are cleared before execution. Uninstrumented targets compile out the timers, and speedup measurements use those uninstrumented targets. Reporting, allocation, and uninstrumented coordination are not attributed to individual phases; phase totals should not be treated as a complete allocator or memory profile.

Profiling the six-factor expression identified dispatch maintenance as a larger cost than binding enumeration. This motivated the selective activation and scope-intersection changes. Instrumented durations include measurement overhead and are diagnostic, not the reported end-to-end speedups.

## Measurement

Measurements compare the baseline with this implementation on an Apple M5 Max running macOS 26.6.2. Both use optimized native Bazel execution, sequential paired runs with alternating revision order, and at least 100 ms warmup per case. Path cases use seven samples. The timings below exclude parsing, reporting, and release. Raw samples, arguments, source digest, counters, and profiling output are in [implementation.json](implementation.json).

| Workload | Before execution (ms) | After execution (ms) | Execution speedup | Including initialization |
| --- | ---: | ---: | ---: | ---: |
| Six factors of two | 72.062 | 69.253 | 1.04× | 1.04× |
| Ten-digit decimal addition | 204.438 | 202.486 | 1.01× | 1.01× |
| Five-digit decimal multiplication | 230.449 | 225.127 | 1.02× | 1.02× |
| 64 rules, 256-token shared fragment | 15.242 | 3.400 | 4.48× | 3.44× |
| 64 rules, 4,096-token shared fragment | 171.540 | 35.151 | 4.88× | 3.41× |
| 4,096 unchanged distinct matchers | 7.969 | 8.047 | 0.99× | 0.97× |
| 4,096 repeated tokens, unchanged domain | 4.012 | 3.964 | 1.01× | 0.97× |
| Thirty factors of two | 3237.509 | 3228.864 | 1.00× | 1.00× |
| 256 rules, 4,096-token shared fragment | 592.616 | 60.462 | 9.80× | 4.87× |

All measured path transition and logical work counts agree. Across 19 exhaustive reference cases, the median per-case speedup is 0.99×, with a range of 0.96–1.04×. Their state, event, work, retained-record, and peak-record counts also agree. Small differences around 1× do not establish a reliable arithmetic or general proof-search improvement.

The fragment benchmark keeps one changing particle and distinct branch worlds. Each rule shares a repeated-token requirement that is impossible by one occurrence; an ordinary stage rule rewrites the changing particle 100 times. It exercises generic multiplicity, invalidation, shared compilation, and posting maintenance. The large speedup is specific to that workload, not an arithmetic throughput claim.

Reproduce the shared-fragment measurements with:

This benchmark was removed after `0daf296` and still runs at that commit:

- [`fragment`](https://github.com/Vantle/photonic/blob/0daf296e1d126675ed445bfb7ed6674a379b1532/benchmark/fragment.rs) with `--width 256 --size 4096 --length 100 --sample 7`

## Remaining boundary

The following remain unimplemented: shared arbitrary multi-particle subplans; maintenance of every surviving partial join product after a relevant domain change; general worst-case-optimal join planning; fully dynamic cyclic reachability; cross-context rewrite-result reuse; causal history reduction; generic instruction fusion; and additional parallel scheduling. These require their own semantic and performance gates. Current results do not justify a universal orders-of-magnitude claim for arithmetic.

Generic creation of new rule shapes remains the separate language proposal in [dynamic.md](dynamic.md). These changes test runtime production and execution of existing code shapes; they do not claim to implement structural reflection or cyclic code values.
