# Compact canonical resource mappings

Baseline: `8dc0182`. The [shared report text audit](render.md) reduced owned report storage, leaving cached canonical forms as a major source of retained engine memory. This change retains the same forms and identity mappings in a smaller representation. Rules, programs, execution order, work accounting, cache lifetime and public interfaces are unchanged.

## Attribution

A temporary diagnostic drops individual components of the direct path's cached canonical forms after reporting and serialization. It samples the existing requested-allocation counter between drops. The six-factor expression releases 63,699,224 bytes from canonical worlds, 22,904,392 from canonical frames, 6,658,144 from world/frame mappings, and 41,877,492 from resource mappings. Together, those components account for 135.1 MB. Cached rendered events release another 2.7 MB. The remaining roughly 31 MB includes execution history, the active evaluator, compiled program and other storage; it is not attributed further here.

The [raw audit](mapping.json) retains the temporary diagnostic patch and both identical observations. That destructive diagnostic runs separately from all timing and acceptance measurements and is removed from production. Its categories describe bytes actually released in the recorded drop order, including shared ownership effects. These are requested Rust heap bytes, excluding allocator metadata, stack storage and process RSS.

## Representation

Canonical resource mappings are immutable after construction. They now use the existing `relation::Map`, which stores sorted key/value pairs contiguously and searches by binary lookup. The temporary incidence table still uses a hash map while collecting repeated occurrences. Canonical ordering still compares the symbol, remapped capture, full occurrence sequence and original resource identifier in exactly the same order. Sorting the finished mapping by its original key changes its storage order, not any assigned identifier.

Mapping storage depends on the number of resources, not the largest identifier. Sparse keys, zero and `usize::MAX` remain valid. No dense identifier assumption, sentinel, cache policy, custom container or dependency is introduced. The wider temporary occurrence records are borrowed while collecting the smaller pairs, and leave scope before canonical particles are constructed; their allocation is not retained by the finished mapping.

This trades expected constant-time hash lookup for logarithmic binary lookup and adds key sorting during construction. It removes retained hash-table capacity and control storage. Timing acceptance therefore covers full reporting, compact execution, small states, wide states, repeated inspection and symmetric canonicalization, rather than assuming that lower memory must imply faster execution.

## Verification and measurement

Optimized Bazel executables run serially on the current Apple M5 Max on AC power after builds and tests finish. Three rounds alternate baseline/candidate order. Each lifecycle process records a cold evaluation separately, followed by five fresh-instance samples. Compact execution and inspection use nine samples. Symmetry uses the existing 100 ms warm-up and 25 samples. Allocation diagnostics run separately from timing acceptance. The raw audit includes all samples, source/executable hashes, fixtures, reproduction scripts and the preliminary comparison.

| Six-factor retained storage | Baseline | Candidate |
| --- | ---: | ---: |
| Engine | 168,795,879 bytes | 150,092,439 bytes |
| Owned report | 107,795,770 bytes | 107,795,770 bytes |
| Encoded buffer | 134,217,728 bytes | 134,217,728 bytes |
| Peak live requested storage | 410,809,377 bytes | 392,105,937 bytes |

The engine retains 18.7 MB less storage, an 11.1% reduction. Whole-lifecycle peak falls 4.6%. Every allocation diagnostic returns to zero additional tracked retained bytes after release. The 512-world controls also retain smaller mappings, but their peak occurs during temporary construction and falls by only 48 bytes; peak savings are workload dependent.

| Workload | Baseline | Candidate |
| --- | ---: | ---: |
| Six-factor full reporting | 447.561 ms | 445.880 ms |
| Six-factor complete lifecycle | 614.050 ms | 615.098 ms |
| Three-factor complete lifecycle | 121.616 ms | 122.539 ms |
| Nested complete lifecycle | 184.250 ms | 182.733 ms |
| Compact six-factor execution | 62.363 ms | 62.557 ms |
| Ten-digit decimal addition | 184.928 ms | 184.027 ms |
| Five-digit decimal multiplication | 207.898 ms | 206.791 ms |
| Incremental inspection | 5.614 ms | 5.454 ms |
| Thirty-world ring canonicalization | 2.240 ms | 2.014 ms |

All 26 protected comparisons pass the existing 5% regression tolerance, or 10% below one millisecond. The matrix includes ten complete lifecycles, three compact arithmetic controls, incremental inspection and twelve symmetry cases. Complete arithmetic latency is approximately unchanged; lower retained storage justifies this change. Some cases improve and some slow slightly. These measurements do not establish a universal lookup or execution speedup.

A new regression relabels shared world and held resources with widely separated identities including zero and the maximum machine word. It checks the complete canonical state and each original-to-canonical identity mapping. Existing coverage includes exhaustive relation-map behavior, 512 sharing graphs, world/token permutations, captured resources, unreachable frames, and canonical scheduling. All 109 optimized Bazel test targets pass, with 105 executed freshly and four cached. The suite includes historical differential evaluation and native/WebAssembly conformance. Formatting and build-integrated checks pass.

Sixteen complete and paused command reports compare byte-for-byte against the baseline, without filtering any fields. State/event/work counts, canonical search steps and serialized observations agree throughout the benchmark matrix. This representation change preserves even the historical execution order, although the broader roadmap permits order changes that preserve execution semantics and the public contract.

Canonical worlds and frames remain the largest attributed engine components. Compact mappings do not compress execution history or remove the cost of producing a complete report. The CLI already streams JSON; the benchmark's retained encoded buffer represents consumers requiring an in-memory serialized value.

```sh
bazel test -c opt //...
bazel run -c opt //:format -- --check
bazel run -c opt //benchmark:lifecycle -- expression '2*2*2*2*2*2' 2101 --sample 5
bazel run -c opt //benchmark:allocation -- expression '2*2*2*2*2*2' 2101 --sample 3
bazel run -c opt //benchmark:symmetry
```
