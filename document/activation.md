# Subscription activation

Baseline: `8356285`. The next architectural opportunity is separating subscription membership from active enumeration. The first bounded slice defers a direct enumeration cursor while any candidate domain is empty. It preserves the original rules, candidate-domain maintenance, traversal order, strategy admission and logical accounting.

## Lifecycle evidence

A separate count-only diagnostic of `2*2*2*2*2*2` records 66,492 search admissions during 10,101 firings. The warmup and measured execution produce identical lifecycle records. Diagnostic logging is excluded from latency acceptance.

| Observation during execution | Count |
| --- | ---: |
| Search admissions | 66,492 |
| Admissions with an empty candidate domain | 56,606 |
| Viable at admission | 9,881 |
| Visited by dispatch during their lifetime | 10,082 |
| Candidate members constructed at admission | 25,235 |
| Those members in searches never visited | 13,422 |
| Removed in the following generation | 49,770 |
| Removed while the input was globally disabled and its frame still present | 53,307 |
| Removed while both the input was disabled and its frame absent | 10,743 |

About 85% of admitted searches initially have an empty domain. The remaining nonviable admissions can fail the distinct-world assignment check despite having nonempty domains. Those cases are semantically different: direct matching can still expose pending steps. Deferral therefore checks domain emptiness, not the broader viability result.

The removal observations describe simultaneous conditions, not exclusive causal attribution. Repeated frame/input/owner coordinates do not prove that a context or its resource occurrences are unchanged. This evidence supports avoiding unnecessary setup; it does not authorize retaining completed results across removal.

## Representation

[Join](../language/joining.rs) keeps its query context, candidate domains, dependency maintenance and ordering. Its traversal can initially contain just the cursor width. Reset materializes the direct cursor when all domains are nonempty. A cursor already activated retains its capacity through later empty domains unless the existing order or strategy replacement path replaces it.

The deferred state charges the same two logical records per input as a reset direct cursor. Physical allocation changes therefore do not alter public record budgets. Strategy admission treats deferred and direct traversal identically, including previously supported product admission while a domain is empty. Shared candidate snapshots, fragment caches, occurrence checks and replay ownership retain their existing boundaries.

An empty input pattern is not an empty candidate domain. `() [] A` can produce `A`; `(),() [,] A` merges two empty coherences into one containing `A`. Each input slot selects a distinct coherence and requires only its specified particles, so an empty slot also matches a nonempty coherence and preserves unmatched particles. The same mechanism handles higher arities and mixed empty/nonempty slots. A program containing only `[] A` starts with no coherences and has no firing in the baseline or candidate. This change preserves that distinction.

Active direct cursors are boxed, making the traversal representation compact alongside the already boxed factored variants. This trades an allocation on activation for less inline storage and movement in every subscription. An inline deferred-cursor alternative was also measured; its preliminary expression result was 1.8% slower than the baseline, with addition 1.1% faster and multiplication 2.3% faster. It was removed. Both experiments are retained in the audit.

No subscription is retained after removal, and no second invalidation system is introduced. This is a small lifecycle slice; it does not eliminate the 66,492 admissions or the work of constructing and maintaining their candidate domains.

## Measurement and verification

The [raw audit](activation.json) retains executable and source hashes, paired samples, allocation diagnostics, the temporary diagnostic patch and verified counts, the alternative representation, fixtures and reproduction scripts. Builds and tests complete before serial timing runs on the current Apple M5 Max. Three rounds alternate baseline and candidate order, using nine samples for compact arithmetic.

| Complete compact execution | Baseline | Candidate | Change |
| --- | ---: | ---: | ---: |
| Six-factor expression | 61.55 ms | 59.55 ms | 3.3% faster |
| Ten-digit decimal addition | 180.50 ms | 179.21 ms | 0.7% faster |
| Five-digit decimal multiplication | 203.81 ms | 198.85 ms | 2.4% faster |

Six-factor execution allocation calls increase from 542,949 to 552,158, about 1.7%. Requested allocation traffic increases from 81.77 to 83.53 MB, about 2.2%. Complete owned-export peak is essentially unchanged at 372.25 versus 372.22 MB. This is a modest execution improvement, not an allocation or export-memory improvement. All allocation diagnostics return to zero additional tracked retained bytes after release. Requested heap bytes do not measure process RSS or the WebAssembly heap.

The separate instrumented profile records the same 66,492 admissions. Median subscription time falls from about 24.27 to 22.89 ms, with preparation falling from 9.20 to 8.14 ms. Instrumented total execution is approximately flat at 68.88 versus 69.13 ms; nested phase times overlap and do not determine acceptance. Dispatch remains the principal measured opportunity, while matching is about 2 ms.

All 42 protected comparisons pass after one calibrated follow-up. The original 27-sample dispatch control measured 136.04 versus 151.38 microseconds, exceeding its 10% submillisecond tolerance. Three alternating rounds with 501 samples per process measure 125.50 versus 125.79 microseconds, within the same tolerance. The original failure and every follow-up sample are retained. The other 41 comparisons pass their original measurements. No timing tolerance changes.

All 109 optimized Bazel test targets pass, including native/WebAssembly conformance. The kernel now contains 228 passing tests. The new activation differential compares every pending/binding result and logical retained count against eagerly materialized cursors through activation, removal, occurrence replacement and eviction. Boundary tests distinguish empty domains, nonempty domains without a distinct-world assignment, and the raw zero-input join.

The separate empty-input tests cover one through six input coherences and zero through four outputs, including empty outputs, every position of a required particle among empty slots, unmatched-particle preservation and insufficient coherence counts. Positive cases run both direct-path and exhaustive execution with small resume budgets. Twenty-two existing complete and paused command reports, plus 60 empty-input and mixed-input reports, match the baseline byte-for-byte. The 128-step dispatch trace also agrees exactly.

```sh
bazel test -c opt //...
bazel run -c opt //:format -- --check
bazel run -c opt //benchmark:expression -- '2*2*2*2*2*2' 2101 --sample 9
bazel run -c opt //benchmark:allocation -- expression '2*2*2*2*2*2' 2101 --sample 3
```

## Next architectural slice

The subsequent [subscription retention experiment](subscription.md) shows why this must separate membership from candidate state. A bounded whole-search cache achieves many potential hits but requires repeated updates while queries are inactive; its actual prototype is about 83–93% slower on four complete expression workloads and was removed. Do not treat the admission counts below as authorization for eager dormant-query maintenance.

Keep coherent subscription membership as the first priority. Investigate how the existing availability, reverse lexical dependency and frame-local registry layers can update affected memberships directly, avoiding rediscovery and reconstruction of a full requested consumer list. Separate lightweight membership and exact dependency state from enumeration that only an active search needs. Do not add an independent registry or cache keyed only by recycled numeric coordinates.

Validate one bounded family before broadening it. Cover global enable/disable transitions, captures, context replacement, new consumers, occurrence reuse and eviction. Preserve exact logical accounting and every interrupted continuation. Measure cold installation, sustained activation churn and complete program execution, including release. Substantial new reuse machinery should target at least 10% lower complete execution latency; current diagnostics are not a speedup forecast.

Shared maintained interior relations remain the next conditional scaling project when two actual consumers repeat a useful join. Lazy parent-trace composition remains conditional on long traces with repeated composition. Neither is justified by the current arithmetic profile alone. Report views are already delivered. The runtime has strong incremental foundations, but these workload-specific results do not establish a globally optimal or universally scalable design.
