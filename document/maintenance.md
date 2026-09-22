# Dispatch maintenance

Baseline: `ec7128d`. Performance acceptance targets the current development machine. Portable builds, exact rule behavior and native/WebAssembly conformance remain required.

## Invariants

A dispatch input is enabled only when its global symbol dependencies are available. Declaration requests use enabled scope entries; executable occurrence requests also require the corresponding input to be enabled. If the enabled set is empty, neither source can produce a request. Returning before lexical ancestry traversal preserves that result. Empty-input rules are initially enabled and cannot be lost through this check. Availability is maintained before reconciliation, so later insertions can activate previously unavailable rules normally. No negative answer is cached.

Agenda reset first clears the agenda and batching cooldown. With zero registered subscriptions there is then no cursor to reset or enqueue. Returning at that point avoids scanning the frame registry. The ordered registry and all of its lookup, insertion, extraction and accounting behavior remain unchanged. A registry containing dormant subscriptions still follows the original path.

These are two existing-state checks, with no new storage, thresholds, rule transformations or progress accounting. On a lexical chain of n present frames with no enabled input, request generation previously traversed the ancestry of each frame: quadratic total ancestry visits. The early return removes those visits. Empty agenda reset no longer depends on the number of allocated registry frames. This does not make all initialization or maintenance constant time; index, context and state work remain.

## Measurement

The dispatch benchmark now reports initialization separately from update execution. Both sides receive the same timer addition. Construction includes program/state creation, indexing and network initialization; update timing retains its previous boundary. The artifact records baseline revision, source hashes, benchmark arguments, alternating rounds, every sample and power state.

The ordinary expression, decimal arithmetic, retained-query, candidate, activation, large-domain, storage and prefix controls use their existing execution measurements. Those measurements do not include complete reporting, destruction or allocator peak bytes. Physical memory and full lifecycle measurement remain queued.

The [raw audit](maintenance.json) contains three alternating rounds per workload, with nine samples per side per round, except the 16,384-frame case, which uses three. All fourteen final workloads preserve every reported non-timing field. The complete 128-update dispatch mutation trace also agrees exactly. Timings use optimized Bazel-built executables sequentially on AC power, after builds and tests finish. Protected tolerances are 5%, or 10% below one millisecond; the final matrix passes. No cross-machine timing gate is applied.

| Workload | Baseline | Candidate | Baseline / candidate |
| --- | ---: | ---: | ---: |
| Six factors of two | 63.664 ms | 64.042 ms | 0.99× |
| Ten-digit decimal addition | 198.246 ms | 195.632 ms | 1.01× |
| Five-digit decimal multiplication | 215.995 ms | 217.909 ms | 0.99× |
| 4,096-frame initialization | 68.653 ms | 1.501 ms | 45.73× |
| 4,096 frames, 128 updates | 0.353 ms | 0.183 ms | 1.93× |
| 16,384-frame initialization | 1480.331 ms | 4.789 ms | 309.11× |
| 16,384 frames, 512 updates | 3.739 ms | 0.566 ms | 6.61× |

The large initialization improvement is specific to a lexical chain with no enabled input. It establishes removal of useless ancestry traversal, not a general arithmetic speedup. The 16-frame initialization control is approximately unchanged. No new cache or allocation strategy is needed for the accepted result.

## Rejected representations

A fully contiguous frame registry improved the initial expression comparison but introduced linear insertion/removal movement in arbitrarily large frames. Large-frame algorithmic behavior did not justify accepting that representation from the small-workload gains alone.

An adaptive array/tree representation bounded those movements. At a 32-entry promotion threshold it did not produce an ordinary-expression gain and roughly doubled the empty large-frame maintenance time. A 128-entry threshold with an empty-frame filter still regressed that control. Both were removed. The original ordered-tree registry remains, avoiding an additional representation, custom traversal and promotion policy.

## Verification

The registry extraction regression compares against an ordinary ordered map through reverse insertion, several frame sizes, bounded partial extraction and predicates that mutate retained values. The dispatcher regression repeatedly empties and reactivates a registry across 257 frames, leaving a prior delivery partially consumed before the next change. It checks exact fresh resource identity, delivery and storage accounting.

The existing mutation benchmark compares full pending/delivery sequences, captures, owners, selections, work, preparation, reuse and retained accounting with the baseline. All 107 optimized Bazel test targets pass, including kernel, historical differential, program and browser conformance checks; 104 execute freshly and three unaffected targets use cached results. Formatting and lint checks pass through the Bazel build. These establish the tested invariants; they are not a proof for every possible program.

```sh
bazel test -c opt //...
bazel run -c opt //benchmark:dispatch -- --width 4096 --length 128 --sample 9
bazel run -c opt //benchmark:dispatch -- --width 16384 --length 512 --sample 3
bazel run -c opt //benchmark:dispatch -- --verify
bazel run -c opt //benchmark:expression -- '2*2*2*2*2*2' 2101 --sample 9
```
