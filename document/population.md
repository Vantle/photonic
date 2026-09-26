# Persistent occurrence populations

This increment follows [incremental occurrence maintenance](invalidation.md) and the [exhaustive replay correction](transcript.md). The baseline is that combined implementation before changing context storage. [The raw audit](population.json) includes executable and source hashes, a baseline patch against `c115754a3a4e056e2b29bdc567b29477355f8b01`, the implementation patch, fixtures, scripts, measurements, intermediate trials and validation output.

The implementation remains under active performance work. Wide context consumption and arithmetic execution improve, but the small scope execution control misses its phase tolerance. Subscription reconciliation, scope overhead, allocation traffic and complete reporting remain open.

## Storage and ownership

A context owns an immutable occurrence population in an `Arc<[Token]>`. Consumption changes a separately shared membership bitmap. Surviving occurrences keep their backing positions, historical states keep their own membership, and clearing the population releases both references. Iteration, equality, ordering and hashing describe the live token sequence. Different physical selections that produce equal sequences remain equal.

The matching index stores backing positions and reconciles exact membership differences when populations share their backing. Replacement populations rebuild local postings. That replacement is required even when the live contents compare equal: compacting a sparse population can change its internal positions. Equal-content replacement does not invalidate logical visibility or recycle its context site. Changes to actual membership still invalidate the affected lexical contexts.

Consumption still scans live members to select removals, and membership differences scan bitmap words. This is not constant-time deletion. A partially consumed population retains its original backing allocation until its last owner drops it. Cell limits continue to describe live occurrences; the allocation diagnostics separately measure physical storage.

## Fingerprint dependencies

Fingerprint maintenance now owns explicit forward and reverse context dependencies for parent, lexical and captured references. Dependency multiplicity is deduplicated; fingerprint contributions retain the original token multiplicity and hashing definition. Edge changes are grouped by target so a broad update reconstructs each affected adjacency list once.

The three existing fingerprint phases recompute changed contexts and consumers whose preceding-phase hashes changed. Unchanged context populations are no longer traversed to rediscover their dependencies. Unaffected world hash records remain shared. Hashes remain filters for exact state comparison; their definition and the collision fallback are unchanged.

The dependency graph participates in record accounting and eviction. After eviction, context changes use complete fingerprint recomputation without retaining the graph again. Eviction preserves the current fingerprint, releases its charged records, and remains safe through subsequent capture changes and consumption.

There are still broader maintenance costs. Frame hash arrays are copied on frame changes. Surviving world dependencies are scanned when context hashes change, and changed reachability or context colors can require rebuilding the context aggregate. This increment does not establish maintenance proportional to changed edges across the whole runtime.

## Validation

All 111 optimized Bazel test targets pass, including 269 runtime tests, native/WebAssembly conformance and headless Chrome. Formatting passes. Nine complete command reports compare exactly against the baseline, including work counts, states, events, captures and provenance. Benchmark observations agree across variants, cold observations and measured samples.

New population checks compare against ordinary vectors across empty, dense and sparse populations, word boundaries and widths through 4,096. They cover immutable branches, stable positions, bidirectional differences, replacement, equality, ordering and hashing. Index checks compare maintained postings, quantities, candidates and storage accounting with fresh indexes through consumption, equal-content compaction, restoration and clearing.

Fingerprint checks compare incremental updates with an independent full recomputation of the previous formula, as well as fresh indexes. They cover captures, cycles, held resources, lexical changes, world replacement, retirement, slot reuse, broad adjacency changes, record accounting and eviction. A locality check verifies that unrelated world hash records retain their allocation identity.

## Measurements

Optimized native binaries run sequentially on the development Apple M5 Max, on AC power, without concurrent builds or tests. The final matrix uses three alternating rounds: 101 samples per small lifecycle, 51 per wide consumption and availability control, 21 per expression and three per streaming export. A scope follow-up uses 1,001 samples per variant in each of three alternating rounds. Allocation instrumentation and phase profiling run separately. Requested heap bytes are not RSS or WebAssembly heap usage.

| Execution workload | Before | After | Change |
| --- | ---: | ---: | ---: |
| 64 ordinary transitions | 0.133 ms | 0.139 ms | 3.8% slower |
| 32 body entries and returns, follow-up | 0.188 ms | 0.207 ms | 10.3% slower |
| 32 context consumptions | 0.228 ms | 0.185 ms | 19.0% faster |
| 256 staged context consumptions | 11.767 ms | 1.853 ms | 6.35× faster |
| `2+2` | 10.258 ms | 9.880 ms | 3.7% faster |
| `2*2*2*2*2*2` | 89.534 ms | 85.501 ms | 4.5% faster |
| `(2+2)*(2+2)` | 34.874 ms | 33.165 ms | 4.9% faster |

The availability execution controls remain within 1% of baseline. The complete wide-consumption lifecycle improves from 29.116 to 19.794 ms. Complete six-factor streaming export increases from 2.834 to 2.923 seconds, about 3.1%.

The existing phase tolerance is 5% above one millisecond and 10% below it. Scope execution misses that gate in both the initial matrix and the longer follow-up; its complete lifecycle increases about 4.4% in the follow-up. No tolerance was widened. Intermediate representations also missed the large-export release gate. The final release comparison is 10.395 versus 10.776 ms, within 5%, but the earlier failures remain in the raw audit. These observations do not establish that every protected phase passes.

Wide-consumption execution allocation traffic falls from 9.42 to 3.50 MB, retained engine storage from 11.37 to 7.46 MB, and complete-lifecycle peak requested storage from 21.74 to 17.84 MB. Six-factor execution allocation traffic instead rises from 102.43 to 124.80 MB. Its streaming-export peak is approximately flat at 439.48 versus 440.21 MB. The scope control also allocates more during execution. Every measured allocation lifecycle releases its tracked allocations to zero.

A separate five-sample profile attributes median fingerprint time of 15.95 versus 5.31 ms, while dispatch remains about 49 ms. Nested phase times overlap, and instrumentation changes timing; these values identify remaining work rather than predict another speedup.

## Intermediate trials and remaining work

The initial bitmap implementation improved wide consumption but regressed the three arithmetic workloads about 5–7%. Profiling located the regression in repeated dependency discovery during fingerprint maintenance. The explicit dependency graph removes that repeated traversal. Subsequent trials batched adjacency updates, replaced shared vectors with immutable slices, added graph eviction, and corrected semantic equality and physical posting replacement. Only the final matrix measures the complete implementation.

The next execution work should maintain subscription consumers from owner changes instead of reconstructing complete request lists. That work must also address the scope control and avoid increasing allocation traffic merely to preserve more cached objects. Reporting remains a separate dominant cost for complete-history consumers. The existing eager retention and broader replay experiments remain rejected; this increment does not reopen them without a distinct measured benefit.

```sh
bazel test -c opt //... //book:check
bazel run -c opt //:format -- --check
bazel run -c opt //benchmark:expression -- '2*2*2*2*2*2' 2101 --sample 21
bazel run -c opt //benchmark:scope -- --width 256 --depth 16 --length 64 --sample 51
bazel run -c opt //benchmark:allocation -- expression '2*2*2*2*2*2' 2101 --sample 2 --export view --writer
```
