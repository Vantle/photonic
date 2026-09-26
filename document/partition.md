# Joined-fragment maintenance

This delivery follows the [dispatch audit](dispatch.md) and baseline `3961d12f80ac7daf72943fdc170d4cfcad763f8b`, which includes the new Buildkite configuration. The remote default branch is `main`; `origin/master` does not exist. The [measurement artifact](partition.json) records the tested source manifest, baseline harness, raw samples, and machine configuration. The [roadmap](roadmap.md) tracks the remaining matching and proof work.

## Implemented boundary

A joined prefix can now retain completed enumeration separately for each candidate occurrence in its first input. Replacing one occurrence retracts that occurrence's fragment while surviving occurrences keep their completed fragments. Inserted candidates are enumerated lazily. Previously, any first-input change invalidated the entire prefix transcript, even when most contributing occurrences survived.

Admission requires an unchanged traversal order, at least two first-input candidates, a surviving candidate, unchanged middle-prefix domains, and the existing wide-pattern/repeated-update gate. Changes to the final suffix remain compatible because the suffix is enumerated against each emitted prefix binding using the current index. A changed middle domain, changed join order, or complete replacement of the first domain falls back to traversal. The existing whole-prefix sharing path remains available for suffix-only changes. Candidate maintenance retains surviving members before appending insertions, so the first member is sufficient to test survival. Admission never performs a Cartesian scan of candidates against insertions.

`joining::Partition` owns the occurrence map, active recording, playback, invalidation, and continuation restoration. `Product<Prefix>` composes either a shared prefix stream or a partitioned prefix with the existing suffix cursor through a private static interface. The outer traversal selects the strategy, allowing each hot path to compile independently. An explicit compact traversal tag avoids extra discriminant decoding on every progress step. `Cursor` exposes an empty-prefix boundary and seeking between first-input candidates. `Trace` owns compressed progress, exact binding identities, and budget reservations. Binding projection retains the existing ordinal resolution and selection ordering. This adds no arithmetic recognition, execution ordering constraint, frontend behavior, or proof-construction shortcut.

This is a bounded joined-fragment implementation, not an arbitrary incremental join graph. A completed fragment includes both bindings and the exact cooperative progress between them. It may represent a completely searched region with no answers. Incomplete recordings are discarded on reset; consumers do not yet share unfinished frontiers or individual partition records across plans.

## Preservation argument

1. At a first-input boundary the prefix cursor has no selected binding. Enumeration until the next such boundary depends on the selected live occurrence, the unchanged remaining prefix domains, immutable prepared patterns/captures, and the existing distinctness and symmetry rules.
2. Surviving occurrences preserve identity and their relative rank. Root insertions/removals therefore cannot change another surviving root's prefix transcript when the other prefix domains are unchanged. If a changed world also belongs to a middle-prefix domain, the conservative invalidation check rejects partition reuse.
3. Every removed site is retracted before the next enumeration, including sites reused numerically in the same update. Equality of stored token values never substitutes for occurrence survival. Recycled worlds are recorded anew even if their contents are identical.
4. Recorded bindings retain stable sites and exact token selections. Current world ordinals are resolved by the existing join boundary. The suffix still performs its own distinctness, symmetry, and resource matching; consumer projection and evidence construction remain outside the cache.
5. Playback emits the same ordered `Pending` and binding sequence as traversal. Crossing a cached partition boundary is internal bookkeeping and emits no extra progress. The final exhausted result remains owned by the source cursor.
6. Eviction during playback releases acceleration data and remembers logical progress within the current partition. The next prefix request reconstructs the physical cursor from that partition's boundary before continuing. Reset discards this restoration because it starts a new enumeration. This also covers eviction while the suffix is active.
7. Cache saturation continues exact traversal. Neither the storage allowance nor recording-length limit truncates the result stream. All admitted recordings are bounded, so restoration has a bounded amount of hidden reconstruction work.

State remains unordered, and gate synchronization still establishes a firing. The traversal order here is an implementation detail whose preservation protects existing progress and suspension behavior; it is not a new language-level ordering rule.

## Accounting and limits

All partitions in one prefix share a 4,096-logical-record allowance, including their trace headers and occurrence keys. They also reserve from the existing shared 65,536-record budget used by matching acceleration. A recording spans at most 65,536 logical steps. A fully exhausted fragment is retained only if both bounds permit it. Removal and dropped recordings return their reservations.

Logical records are not allocator bytes. Cached transcripts consume bounded storage in exchange for saved matching work; peak record counts can differ from the previous implementation. The existing runtime still counts this storage and may evict it under pressure. Required continuations and evidence are not discarded, and production limits are unchanged.

The number of reported logical operations is deliberately unchanged. Avoiding repeated cursor traversal reduces the cost per operation, while replay still delivers each required progress boundary. This compatibility constraint limits the attainable speedup; the audit does not claim an asymptotic reduction in emitted work or a universal arithmetic improvement.

## Measurement

Measurements use optimized native `bazel run -c opt`, sequential alternating baseline/current pairs, AC power, idle-sleep inhibition, and prebuilt binaries. Three rounds provide nine samples per side per scalar workload after warmup. The new benchmark harness is copied unchanged into the isolated baseline; only the current checkout contains the production optimization. Initialization and source construction are outside the focused timer; coherent index updates, join updates, and complete enumeration are inside it.

The partition workload contains several first-input occurrences, a combinatorial middle prefix, and a final suffix. Each update retires and reinserts a configurable number of first-input occurrences. The default middle constraint has no binding; `--productive` exercises exact binding production. Full replacement is a control that admits no surviving partition. This isolates matching maintenance and does not measure an entire expression evaluation.

Protected cases cover arithmetic, existing stable/changing-prefix paths, cross-plan sharing, candidate preparation and activation, leaf reuse, candidate maintenance, and the 19-case exhaustive suite. The predeclared investigation threshold is a repeatable slowdown above 5%, or above 10% for submillisecond samples. Raw timing dispersion and retention are recorded rather than inferred from final answers.

| Workload | Before, ms | After, ms | Ratio |
| --- | ---: | ---: | ---: |
| Partition, width 4 / four roots | 0.2644 | 0.1883 | 1.404× |
| Partition, width 8 / four roots | 5.9010 | 3.2925 | 1.792× |
| Partition, width 12 / four roots | 133.3205 | 68.8400 | 1.937× |
| Partition, width 12 / eight roots | 261.3284 | 100.0583 | 2.612× |
| Productive partition, width 12 | 132.7781 | 69.8600 | 1.901× |
| Complete root replacement | 131.2419 | 131.4345 | 0.999× |
| Shared candidate filters | 10.2168 | 10.1561 | 1.006× |
| Progressive activation | 14.0080 | 14.0039 | 1.000× |
| Stable prefix control | 4.2819 | 4.4996 | 0.952× |
| Changing prefix control | 4.7860 | 5.0038 | 0.956× |
| Six factors of two | 67.6844 | 68.7872 | 0.984× |
| Decimal 1234567890 + 9876543210 | 203.8816 | 203.8459 | 1.000× |
| Decimal 12345 × 67890 | 226.9544 | 228.2578 | 0.994× |
| Leaf reuse | 3.4447 | 3.4652 | 0.994× |
| 4,096-candidate domain | 12.3320 | 12.1132 | 1.018× |
| Stable prefix control, confirmation | 4.2248 | 4.4516 | 0.949× |
| Changing prefix control, confirmation | 4.7175 | 4.9594 | 0.951× |

Ratios above one are faster. The scalar table pools 27 samples per side; the two confirmation rows pool 357 samples per side across seven further alternating pairs. The targeted gain is 1.40–2.61×, including 1.90× when actual bindings are produced. The full-replacement control is unchanged. Six factors of two are about 1.6% slower; decimal addition is unchanged and decimal multiplication is about 0.6% slower.

The protected prefix controls retain a measured cost: 5.37% and 5.13% slower in the longer confirmation. This triggered the declared investigation threshold. Moving prefix strategy selection out of the product hot loop and using an explicit traversal tag removed much larger replay regressions. Additional inlining/projection experiments did not reliably remove the remaining cost and were reverted. The retained implementation is a scoped architectural tradeoff; it is not a regression-free or universal speedup. Reduce that overhead before broadening admission. Existing isolated join cases span 0.946–1.018×; cross-plan sharing cases span 0.994–1.027×.

All paired matching work/binding counts and arithmetic event/work counts agree. The four-root, width-12 negative case emits the same 14,676,800 logical steps per run and retains 707 records versus 693 before. The productive case emits the same 3,072 bindings and 14,725,952 logical steps; its peak retained count rises from 671 to 2,413. These are retained-record observations at update boundaries, not allocator-byte peaks or an every-step memory profile. The eight-root negative case retains 859 versus 833 records.

All 19 exhaustive fixtures retain identical state, event, work, record, and peak counts in both ordinary and instrumented runs. Median per-case timing ratios are 0.991× and 0.992× respectively; their ranges are 0.942–0.999× and 0.972–1.009×. The focused partition speedups do not apply to exhaustive proof execution.

Reproduce the focused workloads with:

This benchmark was removed after `0daf296` and still runs at that commit:

- [`partition`](https://github.com/Vantle/photonic/blob/0daf296e1d126675ed445bfb7ed6674a379b1532/benchmark/partition.rs) with `--width 12 --sample 9`, `--width 12 --count 8 --sample 9`, `--width 12 --productive --sample 9` and `--width 12 --replacement 4 --sample 9`

## Validation

Five new kernel tests compare every progress result and complete binding with direct traversal. They cover migration from whole-prefix sharing, survivor retention, changed token identities at reused sites, insertion/deletion without join reordering, mutation of other inputs, nested rule values, partial enumeration, reset, small/global cache budgets, eviction at 128 replay offsets, and both storage and recording-length saturation. Independent retained-size checks run at each compared step; dropping and evicting caches must return all global reservations.

`bazel test --nocache_test_results //language:test` and `bazel test -c opt --nocache_test_results //...` pass. The kernel has 166 tests; the complete optimized suite has 106 targets, including native/WASM conformance and the memory-heavy repeated ternary test. The configured Rust and Bazel formatting/lint aspects remain enabled.

## Remaining work

The next matching extension is explicit dependency ownership for fragments below the first input, allowing a changed middle occurrence to retract only the fragments that depend on it. It needs exact consumed/read/capture dependencies and independent delivery positions, not merely a smaller cache key. Admission should remain performance driven, especially for consumers that stop before completing a fragment.

Shared unfinished enumeration, cross-plan partition sharing, arbitrary join-fragment graphs, exhaustive-proof integration, contextual rewrite reuse, and GPU execution remain unfinished. This delivery establishes the first occurrence-maintained joined fragment and its bounded fallback path. Browser execution timing, allocator peak bytes, Windows execution, and GPU crossover measurements are outside this audit.
