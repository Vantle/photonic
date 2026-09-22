# Resource bounds from population changes

This increment follows [incremental capture dependencies](dependency.md). Layout maintenance now derives resource bounds from changed owned occurrences when populations share immutable backing contents. The [raw audit](identity.json) records source patches, executable hashes, paired native measurements, allocation counters, isolated scaling measurements, exact reports, validation and rejected implementations.

## Invariant and structure

A layout's resource bound is one greater than the largest resource identity in physical state storage, or zero when there are no resources. This includes unreachable frames. The bound can decrease after consumption or retirement; it is not a monotonically increasing allocation counter.

The new resource component computes a bound for each changed frame. For shared owned populations, it examines the physical positions present in the current version and absent from the previous version. It uses the existing population-difference operation. Replacements and newly created frames use ordinary full-population traversal. Held resources contribute their complete bound. These frame bounds combine with inserted-world bounds.

If the resulting bound reaches or exceeds the previous resource bound, it is already exact: all unexamined occurrences existed in the previous state. Otherwise, the reverse comparison identifies whether the previous maximum could have disappeared. A full-state scan resolves that case, including identities still present elsewhere. When the previous maximum was untouched, the existing bound remains exact. Duplicate identities, restoration, reused frame slots and arbitrary token order follow the same calculation.

The implementation adds no persistent cache, resource counter or eviction policy. Cell counting and initial layout construction retain their existing formulas. Resource calculations operate on frame-level bounds; ordinary population traversal and membership subtraction remain separate operations beneath that boundary.

## Validation

All 111 optimized Bazel test targets pass, including 283 runtime tests and native/browser conformance. Formatting passes. Nine complete command reports compare exactly with the baseline. All lifecycle observations, output sizes, writer fingerprints, symmetry step counts and repeated-inspection observations agree.

Four new layout tests compare incremental layouts with independent full-state reconstruction. They cover population sizes 0, 1, 2, 63, 64, 65, 129 and 513; repeated removal, shared-backing restoration of the highest identity, replacement, reversal and compaction; duplicate maximum identities across owned, held and world occurrences; `usize::MAX - 1` identities; frame creation, retirement and reuse; and unreachable physical storage. Existing evaluator-oracle, fingerprint, collision, scheduling and reclamation checks remain in the full suite.

## Native measurements

Optimized native executables ran sequentially on the development Apple M5 Max without concurrent builds or tests. The final confirmation uses three alternating rounds, with 201 samples for small and wide lifecycles, 101 for compact expressions and availability, and three for streaming export. The additional 1,024-occurrence fixture uses eleven samples per round. Results are medians of round medians. All final phases and controls pass the existing limits of 5% at or above one millisecond and 10% below it.

| Workload or phase | Before | After | Change |
| --- | ---: | ---: | ---: |
| 256-occurrence consumption execution | 1.545 ms | 1.460 ms | 5.5% faster |
| Complete 256-occurrence lifecycle | 13.621 ms | 13.509 ms | 0.8% faster |
| 1,024-occurrence consumption execution | 15.288 ms | 13.967 ms | 8.6% faster |
| Complete 1,024-occurrence lifecycle | 200.717 ms | 200.461 ms | Approximately flat |
| Compact six-factor execution | 73.733 ms | 73.950 ms | Approximately flat |
| Compact six-factor initialization | 4.192 ms | 4.400 ms | 5.0% slower |
| Six-factor streaming export | 2.349 s | 2.373 s | 1.0% slower |

Compact expression initialization increases about 4.2–5.0%, close to the existing gate. The six-factor ratio is 1.049596, below the 1.05 limit. This is a measured cost of the complete executable, although the initial layout formula is unchanged. Arithmetic execution is approximately flat. The larger consumption fixture spends about 130 ms in reporting and 48 ms in serialization, so its execution improvement barely changes total lifecycle time.

Requested allocation and peak storage are unchanged in ordinary execution, scope entry, both wide fixtures and six-factor export. Small-consumption execution allocates 32 additional bytes. Every measured lifecycle returns tracked retained allocation to zero. The 1,024-occurrence fixture allocates about 1.030 GB cumulatively and peaks at 254.2 MB in both versions. These are Rust allocation counters rather than RSS or WebAssembly heap measurements.

A separate diagnostic profile places the gain in rewriting, which includes layout maintenance. That phase falls from about 0.450 to 0.313 ms for the 256-occurrence fixture and from 5.058 to 3.884 ms for the 1,024-occurrence fixture. Fingerprint maintenance remains about 7.5 ms in the larger case. Instrumented phase timers overlap and are separate from native acceptance.

## Scaling and measurement limits

The isolated layout benchmark prepares state transitions and reachability outside the timer, then measures 32 single-occurrence removals. Population width varies independently across 64, 512, 4,096 and 65,536 occurrences. Each width includes removal below the maximum, repeated removal of the maximum, and replacement with newly allocated backing contents. Initialization samples average 64 constructions to reduce timer quantization. The final sweep uses 301 samples through width 4,096 and 51 at width 65,536, in three alternating rounds.

For a shared population of 4,096 occurrences, updates below the maximum fall from about 87.9 to 5.0 microseconds; at 65,536 occurrences they fall from 1.346 ms to 40.7 microseconds. Repeated maximum removal at 65,536 occurrences falls from 4.339 to 1.582 ms and still requires full-state scans. Replacement controls remain within tolerance. These are isolated metadata improvements; the native table above measures complete execution.

Shared-population difference discovery scans membership words and enumerates changed positions. Its cost still depends on backing width and mutation size. Replacement traverses the affected population, and possible loss of the maximum still invokes a full-state scan. Color recomputation continues to scan surviving occurrences, so this change does not make wide consumption scale linearly overall.

The initial native matrix reported a 6.5% wide-lifecycle regression and expression-initialization regressions just above 5%. Several measurements drifted substantially between rounds. The longer complete confirmation resolves the wide result and puts all expression-initialization results within tolerance, while preserving the near-limit cost above. Early isolated initialization measurements also crossed their limits; batched construction and the longer complete sweep provide the final measurements. Every initial failure is retained in the audit.

## Rejected traversal interfaces

Several attempts exposed one lazy token stream combining membership differences with replacement traversal. They passed correctness tests but slowed replacement about two to five times in the isolated controls. Variants using an explicit iterator enum, forwarding operations and standard iterator composition did not establish acceptable replacement performance. Extracting the complete resource scan into a separate function also regressed initialization in the early measurements; restoring the existing initial-layout form recovered that cost. The evidence demonstrates sensitivity to the traversal boundary without establishing a hardware-level explanation.

The final implementation combines scalar bounds from each population. It retains the existing population interface and removes every experimental iterator, forwarding method and associated production refactor. The rejected source patches and measurements are preserved for future work.

Exact fingerprint summaries, whole-array color copying, canonical graph construction and refinement, and reporting allocation remain open. The larger fixture makes fingerprint maintenance the next measured execution priority: investigate maintaining the owned-symbol aggregate for the initial color round inside the existing population summary. That round depends on symbol multiplicity; later rounds still incorporate changing captured colors. Preserve the current fingerprint formula and measure the new summary across replacement, restoration and eviction. This increment improves resource-bound maintenance and larger consumption execution; it does not complete the runtime roadmap.

```sh
bazel test -c opt //... //toolchain/browser:check --test_output=errors
bazel run -c opt //:format -- --check
bazel run -c opt //benchmark:layout -- --width 65536 --length 32 --sample 51
bazel run -c opt //benchmark:layout -- --width 65536 --length 32 --sample 51 --descending
bazel run -c opt //benchmark:layout -- --width 4096 --length 32 --sample 301 --replacement
bazel run -c opt //benchmark:lifecycle -- expression '2*2*2*2*2*2' 2101 --sample 3 --export view --writer
```
