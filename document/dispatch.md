# Dispatch dependency maintenance

This delivery follows `dfaf48e` and the [candidate-domain audit](candidate.md). The file manifest and paired native measurements in [dispatch.json](dispatch.json) identify the measured source. The [roadmap](roadmap.md) continues to track joined-binding maintenance and the remaining proof work.

## Implemented boundary

Dispatch now shares lexical-ancestry discovery within each coherent update. Previously, each present frame walked its lexical ancestors independently and searched the changed-frame list at every ancestor. `dispatch/context.rs` memoizes whether each visited frame reaches a changed context. Ancestors shared by many present frames are evaluated once. This reduces context selection from repeated depth-dependent walks to linear work in the frame table and changed list, with linear temporary storage. It does not retain a reverse lexical dependency index between updates.

Availability maintenance now processes disappearing symbols before appearing symbols. Consumers observe only the resulting snapshot. A conjunction that was disabled before the update and remains disabled afterward no longer becomes transiently enabled while an unordered changed-symbol set is traversed. Insertions are collected during the first pass, so each changed symbol requires one availability lookup. This ordering belongs to index maintenance; it does not impose an execution order on Photonic rules.

The agenda, matching cursors, candidate filters, joined-prefix caches, resource occurrences, capture identities, and proof construction retain their existing algorithms. No arithmetic operation receives a specialized implementation. The frontend is unchanged.

## Measurement boundary

The native profile now separates subscription reconciliation, consumer requests, query preparation, availability, context discovery, domain refresh, and agenda restart. These are nested scopes: request and preparation are inside subscription, which is inside dispatch. Their durations must not be summed as independent execution phases. Instrumentation is compiled only into the measurement build.

The initial diagnostic of six factors of two attributed roughly 25 ms to subscription work, 4.6 ms to refresh, and 0.22 ms to agenda restart. This made a lazy agenda rewrite a poor first target for that workload. The existing eager restart preserves delivery, suspension, and logical accounting behavior. Further subscription planning and joined-result maintenance remain larger opportunities for arithmetic.

## Preservation argument

1. A frame is affected by a lexical change exactly when its lexical ancestor chain intersects the changed-frame set. The memo table caches that predicate for the current immutable snapshot. It does not infer eligibility from numeric frame order.
2. Every changed frame that still exists starts marked affected. A traversal stops at a previously established answer or at the end of its ancestor chain; every frame on the traversed path receives that same answer. The existing valid lexical ancestry invariant is unchanged.
3. Memoization considers intermediate ancestors without live worlds, as well as present frames. Removed frame identifiers remain in the caller's explicit changed list so their old subscriptions are removed. No answer is retained across frame mutation or slot reuse.
4. The selected frame sequence still follows the index traversal and is combined with the same affected/context sets before their existing sorting and deduplication. Consumer order, binding order, and cooperative progress are preserved.
5. Removing symbols can only disable input conjunctions; adding symbols can only enable them. Completing both phases before subscription reconciliation gives the same final missing counts and enabled inputs, without a temporary activation of an ultimately disabled conjunction.
6. State remains unordered and firings still require exact gate synchronization. No required proof, candidate, or occurrence is suppressed by these changes.

## Accounting

The previous implementation could retain an extra altered-input marker depending on randomized hash iteration order. The mutation audit observed a one-record difference at affected checkpoints even between executions of the old algorithm. This delivery removes those transient markers and avoids subscription reconciliation caused only by a temporary activation. Bookkeeping and cache lifetime can therefore differ from a particular old hash iteration order. In the paired mutation audit, the observed difference was at most one retained record. A lower footprint can allow progress under a record limit that previously counted that unnecessary marker. It does not raise the limit or omit live retained records.

The differential audit compares complete serialized progress and delivery sequences, including rule, frame, owner, read identity, selected worlds, input positions, and token identities. It also compares work, binding, preparation, reuse, and eviction counts. It records retained counts separately because of the intentional bookkeeping correction above.

## Paired native results

Measurements used `bazel run -c opt` on an Apple M5 Max connected to AC power, with idle sleep inhibited. Three rounds alternate baseline/current execution order, with nine samples per workload per round and a warmup. The table uses pooled medians of 27 samples per side. The unchanged benchmark harness was added to the isolated baseline checkout; its file hashes are recorded in the artifact. The context harness uses the instrumented library, including additional nested scopes in the new implementation. Ordinary arithmetic and matching benchmarks use the uninstrumented library.

| Workload | Before, ms | After, ms | Ratio |
| --- | ---: | ---: | ---: |
| Context, 64 frames | 0.1578 | 0.0267 | 5.916× |
| Context, 256 frames | 3.4655 | 0.0520 | 66.698× |
| Context, 1,024 frames | 80.4938 | 0.1881 | 427.970× |
| Shared candidate filters | 10.3906 | 10.2780 | 1.011× |
| 128 rules × 128 terms | 44.4410 | 44.3409 | 1.002× |
| Progressive activation | 14.3202 | 14.3113 | 1.001× |
| Stable prefix | 4.5939 | 4.3396 | 1.059× |
| Changing prefix | 5.0613 | 4.8487 | 1.044× |
| Six factors of two | 68.5041 | 69.0145 | 0.993× |
| Decimal 1234567890 + 9876543210 | 205.3013 | 205.9860 | 0.997× |
| Decimal 12345 × 67890 | 230.3774 | 229.8212 | 1.002× |
| Leaf reuse, initial run | 3.5148 | 3.6002 | 0.976× |
| Candidate domain, 4,096 worlds | 12.4521 | 12.4678 | 0.999× |
| Leaf reuse, confirmation | 3.4915 | 3.4864 | 1.001× |

The large gains apply to the isolated ancestry workload. Arithmetic remains close to its prior speed. The initial short leaf-reuse run suggested a small regression; seven further alternating pairs with 51 samples per side each did not reproduce it. Those 357-sample confirmation measurements are also retained in the artifact. Small timing differences are not evidence of a universal improvement.

All compared direct workloads retained identical event/work counts. All 19 exhaustive fixtures retained identical state, event, work, record, and peak counts in both ordinary and instrumented runs. The mutation harness retained identical complete progress/delivery sequences, work, binding, preparation, reuse, and eviction counts in all three paired rounds.

Reproduce the focused checks with:

```sh
bazel run -c opt //benchmark:dispatch -- --width 1024 --length 32 --sample 9
bazel run -c opt //benchmark:dispatch -- --verify
bazel run -c opt //benchmark:profile -- '2*2*2*2*2*2' 2101 --sample 7
```

## Validation

`bazel test //language:test` passes all 161 kernel tests. `bazel test -c opt --nocache_test_results //...` passes all 106 targets, including Rust/WASM conformance and the configured formatting/lint checks.

- Three new kernel regressions cover ancestry against an independent chain-walking oracle, repeated symbol replacement in a conjunction that must remain disabled, and delayed activation of an existing dormant subscription in another frame. The ancestry matrix includes arbitrary numeric frame order, inactive intermediate ancestors, removed identifiers, and multiple changed frames.
- The native mutation harness performs 128 updates with partial enumeration, periodic complete drains, eviction, executable rule occurrences, captures, lexical changes, empty particles, and recycled sites. Its [complete trace](dispatch.trace.json) is compared against the previous implementation using the same harness. The identical step arrays are stored once; each paired run retains its own counters and retention observations in the main artifact.
- The native benchmark suite checks arithmetic and direct matching event/work counts, plus state/event/work/record/peak counts for 19 exhaustive fixtures.

## Remaining boundary

This is the dispatch dependency-maintenance milestone. It does not complete a persistent agenda, per-frame subscription index, individual joined-result retraction, shared unfinished enumeration, exhaustive-fragment integration, contextual rewrite reuse, or GPU execution. The next substantial matching milestone remains one bounded joined fragment with occurrence dependencies and exact insertion/retraction behavior.

The deep-context benchmark isolates ancestry maintenance: it creates a chain whose rule cannot activate, then changes a separate leaf's lexical ownership 32 times. Timed execution includes snapshot/index updates, dispatch, and checking for completion; initial program/network construction is outside that interval. It demonstrates the removed repeated traversal, not a speedup for every deeply nested program. Arithmetic and other workload measurements are reported separately.

Native Linux/Windows execution, browser timing, GPU crossover measurements, and allocator peak-byte measurements are outside this audit. Native and WebAssembly conformance tests run locally through Bazel.
