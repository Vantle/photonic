# Incremental capture dependencies

This increment follows [shared canonical incidence](incidence.md). Fingerprint contexts now share an immutable dependency graph, and repeated consumption maintains capture multiplicity from population changes. The fingerprint definition remains unchanged. The [raw audit](dependency.json) contains source patches, executable hashes, paired measurements, allocation counters, diagnostic profiles, rejected variants and validation evidence.

## Structure and ownership

An owned population already separates immutable token contents from live membership. A dependency cache retains that population and, after an update reuses its backing contents, materializes a persistent ordered map from captured frame to occurrence count. Subsequent removals decrement the corresponding counts; restoration increments them. A dependency disappears only after its final occurrence disappears. New backing contents discard the counts and use direct enumeration until reuse justifies materialization.

A separate persistent map stores these population caches only for frames with owned particles. Empty frames have no population cache entry. Parent, lexical and held-token references compose with the owned capture targets to produce each frame's outgoing dependency set. Incoming adjacency changes only when that set changes, preserving references that remain supported by another kind of edge. Retired frames remove their population entries.

Contexts share the complete immutable dependency graph through `Arc`. A transition that changes no frame reuses it; a changed transition constructs one new graph. This removes the earlier clone of a graph that was immediately replaced by another updated clone. Persistent maps preserve historical indexes without duplicating every counter on each update.

The existing three color rounds and their hash formula are unchanged. In particular, captured colors still participate in the same nonlinear token hash; this increment does not replace that formula with a cheaper approximation. Logical retention includes each occupied population cache and each materialized capture-count entry. Eviction releases the graph and its population caches together, and the existing full-recomputation fallback remains exact.

## Validation

All 111 optimized Bazel test targets pass, including 279 runtime tests and native/browser conformance. Formatting passes. Nine complete command reports compare exactly with the baseline. Lifecycle observations, streaming byte counts, writer fingerprints, symmetry step counts and repeated-inspection observations agree. Every measured lifecycle returns tracked retained allocation to zero.

The fingerprint tests compare incremental results with an independent implementation of the original hash formula, as well as fresh indexes. Population tests cover sizes 0, 1, 2, 64, 65 and 257; repeated removal, shared-backing restoration, replacement, compaction, reversal, uncaptured atoms and repeated capture targets. Historical indexes remain unchanged. A separate multiplicity test removes owned, held, parent and lexical support in stages, checking that the edge disappears only after the final support is removed. Existing retirement, slot reuse, cyclic capture, world replacement and eviction cases remain in the suite.

## Native measurements

Optimized native executables ran sequentially on the development Apple M5 Max without concurrent builds or tests. Three alternating rounds use 101 samples for small lifecycles, 51 for wide consumption and availability, 21 for compact expressions and three for streaming export. Values below are medians of the round medians. All protected phases and controls pass the existing limits of 5% at or above one millisecond and 10% below it.

| Workload or phase | Before | After | Change |
| --- | ---: | ---: | ---: |
| Wide-consumption execution | 1.890 ms | 1.590 ms | 15.9% faster |
| Complete wide-consumption lifecycle | 14.203 ms | 13.689 ms | 3.6% faster |
| Compact six-factor execution | 73.782 ms | 73.761 ms | Approximately flat |
| Six-factor streaming export | 2.337 s | 2.351 s | 0.6% slower |
| Complete six-factor streaming lifecycle | 2.430 s | 2.443 s | 0.5% slower |
| Repeated inspection | 14.719 ms | 14.698 ms | Approximately flat |

Small lifecycle totals improve about 0.4–2.3%. Compact expression initialization improves about 5–6%, while execution remains approximately flat. These results establish a wide-consumption improvement, not a general arithmetic speedup.

Wide execution's cumulative requested allocation falls from 3.252 to 2.429 MB, a 25.3% reduction. Six-factor execution falls from 117.303 to 107.652 MB, an 8.2% reduction. Whole-lifecycle allocation changes from 70.215 to 69.394 MB for wide consumption and from 10.197 to 10.187 GB for six-factor export. Peak requested storage is approximately unchanged, increasing by less than one kilobyte in each measured lifecycle.

The cache has a measurable cost on smaller workloads: scope execution allocation rises from 450.526 to 483.806 kB, about 7.4%; small-consumption execution rises from 365.078 to 387.478 kB, about 6.1%. Their complete lifecycles allocate about 1.6% and 0.9% more, respectively. Ordinary execution allocation is unchanged. Counters measure requested Rust heap bytes, not RSS or WebAssembly heap size.

The separate diagnostic profile attributes wide consumption’s improvement to dependency discovery: that phase falls from about 0.361 to 0.048 ms, while total fingerprint maintenance falls from 0.871 to 0.600 ms. Color recomputation remains about 0.43–0.46 ms. In six-factor execution, dependency bookkeeping increases from 1.705 to 2.031 ms, but total fingerprint maintenance falls from 5.684 to 4.982 ms. The graph-sharing change removes work outside dependency discovery; the cache itself is not cheaper on every workload. Nested phase timers overlap and are separate from native acceptance.

## Scaling and rejected variants

A focused benchmark prepares states and layouts outside the timer, then advances fingerprint metadata through 32 single-occurrence removals. It varies population width and capture-target count independently and includes replacement with newly allocated backing contents. Both versions use the same diagnostic instrumentation. These measurements establish metadata scaling rather than whole-program throughput.

With shared backing, 512 occurrences capturing one or eight frames improve update time about 30% and 41%, respectively; 4,096 occurrences capturing 32 frames improve about 47%. With 512 distinct capture targets, updates slow about 8.8% at submillisecond scale. Replacement is approximately flat or slightly faster, and initialization stays within about 1% of baseline. Final focused measurements use three alternating rounds of 101 samples. An initial 11-sample run reported a five-microsecond initialization cost above tolerance; the complete longer sweep resolves that variance without changing the tolerance. Both runs are retained.

The first eager counter implementation copied ordinary ordered maps and materialized counts on every replacement. It passed the native lifecycle matrix but made the 512-target replacement control about 3.2 times slower. Persistent counters and admission after backing reuse removed that failure. A later representation allocated population records for empty frames; another retained an optional slot for every frame. Both were replaced by the final sparse persistent map.

The optional-slot version also made repeated inspection about 5.5% slower in its original three rounds; a longer follow-up measured about 4.6%. The final representation measures approximately flat. Repeated inspection uses the exhaustive engine and does not call the changed incremental fingerprint path, so the evidence does not establish a direct algorithmic explanation for that executable-level difference. The earlier failure and follow-up remain in the audit rather than being discarded.

## Remaining work

Resource identity maintenance still scans complete changed populations, and color recomputation still copies whole frame arrays. Six-factor execution allocation remains above the earlier pre-population baseline of about 102.4 MB. Canonical incidence construction and refinement remain major reporting costs. This increment closes a measured dependency-maintenance cost; it does not establish completion of the runtime roadmap.

```sh
bazel test -c opt //... //toolchain/browser:check --test_output=errors
bazel run -c opt //:format -- --check
bazel run -c opt //benchmark:lifecycle -- expression '2*2*2*2*2*2' 2101 --sample 3 --export view --writer
bazel run -c opt //benchmark:allocation -- expression '2*2*2*2*2*2' 2101 --sample 2 --export view --writer
bazel run -c opt //benchmark:inspection -- --length 32 --sample 21
```

This benchmark was removed after `0daf296` and still runs at that commit:

- [`fingerprint`](https://github.com/Vantle/photonic/blob/0daf296e1d126675ed445bfb7ed6674a379b1532/benchmark/fingerprint.rs) with `--width 4096 --depth 32 --length 32 --sample 101` and `--width 512 --depth 512 --length 32 --sample 101 --replacement`
