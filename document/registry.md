# Compact subscription keys

Baseline: `3b13457`. The refreshed compact-expression profile still spends approximately 41 ms in dispatch within a 72 ms instrumented execution, including 27 ms in subscription reconciliation. Index maintenance takes about 13 ms and matching about 2 ms. These scopes overlap; preparation and admission are nested within subscription work. This change reduces redundant storage in the ordered subscription registry.

## Representation and invariants

The registry already owns a separate ordered tree for each frame. Each stored key now contains only its input and owner. The enclosing vector supplies the frame identifier. On the measured 64-bit target, the key occupies 16 bytes instead of 24. The implementation retains standard balanced trees and their logarithmic lookup, insertion and removal behavior; it introduces no threshold, promotion policy or cache.

Registry boundaries reconstruct the complete frame/input/owner key. Iteration visits frame slots in ascending order, then input/owner pairs in tree order, reproducing the original lexicographic order. Iterators return a small owned key instead of borrowing a redundant stored key; this reconstruction allocates no heap storage. Lookup selects the same frame tree before comparing the remaining coordinates. Ranges still assert that both endpoints belong to one frame, and inclusive bounds retain the full machine-word range.

Extraction passes the reconstructed key to the existing predicate and decrements the registry count only when the underlying tree yields a removed entry. Partial consumption and predicates that mutate retained values retain their behavior. Replacement, total counts, per-frame counts, agenda ordering, preparation/reuse statistics and logical retained-record accounting are unchanged. This is an internal interface change; public reports, rules and programs are unchanged.

## Measurement

The [raw audit](registry.json) includes source/executable hashes, machine and power state, all preliminary and final samples, instrumented profiles, allocator results, fixtures and reproduction scripts. Optimized Bazel executables run serially on the current Apple M5 Max on AC power after builds and tests finish. Three rounds alternate baseline/candidate order. Allocation and instrumented profile runs are separate from uninstrumented timing acceptance.

Lifecycle processes record a cold evaluation separately and then five fresh-instance samples. Compact arithmetic and ordinary maintenance controls use nine samples per process. The 16,384-frame control uses three. Symmetry retains its existing 100 ms warm-up and 25 samples. All non-timing observations agree.

| Workload | Baseline | Candidate |
| --- | ---: | ---: |
| Compact six-factor execution | 63.288 ms | 62.214 ms |
| Ten-digit decimal addition | 187.877 ms | 184.216 ms |
| Five-digit decimal multiplication | 211.413 ms | 207.769 ms |
| Six-factor complete lifecycle | 619.680 ms | 621.610 ms |

The final compact arithmetic medians improve approximately 1.7–2.0%. Full-export latency is approximately unchanged. Preliminary multiplication samples regressed about 4%, so the result is a modest measured improvement rather than a universal speed claim. The final matrix passes all 42 protected comparisons at the existing 5% regression tolerance, or 10% below one millisecond. Those comparisons include complete reporting, arithmetic, inspection, symmetry, retained queries, large domains, executable-rule storage, candidate activation, changing prefixes, and dispatch initialization/update at several frame counts.

In the six-factor allocation diagnostic, execution requests 89,067,991 bytes before and 88,518,239 after, approximately 0.55 MB less allocation traffic. Whole-lifecycle peak remains approximately 372.4 MB; the small registry saving does not materially change a peak dominated by canonical forms, owned reports and the encoded buffer. Every allocation diagnostic returns to zero additional tracked retained bytes after release. Requested allocation counters exclude allocator metadata, stack storage and process RSS.

## Verification and remaining work

The existing registry oracle checks 4,096 mutations against an ordinary full-key ordered map, including replacements, lookups, iteration, bounded range extraction, partial consumption and predicates that mutate values. Coverage now includes `usize::MAX` input and owner identifiers. This checks the coordinate conversion at the largest inclusive boundaries.

All 109 optimized Bazel test targets pass, with 105 executed freshly and four cached. The language target passes again after the boundary coverage expansion. The suite includes historical differential evaluation and native/WebAssembly conformance. Sixteen complete and paused command reports compare byte-for-byte, and the complete dispatch mutation trace agrees with the baseline. Formatting and build-integrated checks pass.

The larger dispatch opportunity remains repeated subscription construction and preparation. This change does not eliminate that work or alter execution order. Further reuse needs a concrete ownership and invalidation argument plus cold, changing and low-reuse controls. Adding another matching cache solely because matching can be optimized would miss the measured dominant cost.

```sh
bazel test -c opt //...
bazel run -c opt //:format -- --check
bazel run -c opt //benchmark:profile -- '2*2*2*2*2*2' 2101 --sample 5
bazel run -c opt //benchmark:expression -- '2*2*2*2*2*2' 2101 --sample 9
bazel run -c opt //benchmark:dispatch -- --verify
```
