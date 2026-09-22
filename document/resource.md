# Canonical resource storage

Baseline: `7786acd`. The [canonical reporting audit](canonical.md) removed unnecessary refinement work. This increment reduces allocation calls and repeated resource lookup during canonical state renaming and incidence construction. Existing rules, program behavior, public report fields and canonical identity ordering are preserved.

## Changes and invariants

Renaming records each resource's world and held-frame occurrences before assigning canonical identifiers. That temporary occurrence list now uses the existing `SmallVec` dependency with room for one entry inline. A singleton needs no separate allocation. Longer lists spill to ordinary heap storage; multiplicity, order and capacity are not semantically limited. `SmallVec` compares its contents through slice ordering, so the sort still compares symbol, remapped capture, complete occurrence sequence and original resource identifier in the same order. No dependency, cache or custom container was added.

Incidence construction now stores a resource's graph vertex and optional capture in one resource-index entry. The first encountered occurrence still supplies its label and vertex. Each subsequent nonempty capture updates that entry, preserving the prior last-nonempty-capture behavior. Empty capture observations do not erase an existing capture. A single final traversal emits one capture connection for each captured resource. This removes the second token scan, separate capture hash table and repeated resource-index lookups. Only reachable held frames participate, as before.

Graph capture-edge iteration order may differ because it comes from the resource map rather than a second hash map. That order was already unspecified. Edge kinds, endpoints and multiplicities are preserved, and canonical refinement sorts adjacency signatures. Exact output comparisons verify the final externally visible ordering.

The new resource-index entry is wider. This is an explicit tradeoff: fewer allocations and lookups do not necessarily mean fewer requested bytes. Storage remains bounded by the resources and occurrences in the current state, with no persistent cache or invalidation policy.

## Verification

Existing canonicalization tests enumerate 512 sharing graphs, permute worlds and particle order, and relabel resource identifiers. They include resources present in multiple worlds, exercising heap-spilled occurrence lists. The symmetry benchmark covers private, shared and cyclic resource incidence. Existing kernel tests cover captured imports, held resources and canonical environment reconstruction.

A new regression places one captured resource repeatedly in two worlds and in two held frames, with occurrence counts from 1 through 65. It checks that incidence contains exactly one capture connection and that renaming preserves every occurrence and its capture while assigning one resource identity. A separate unreachable frame contains another captured resource and must remain absent from the incidence graph and canonical result.

All 109 optimized Bazel test targets pass: 105 execute freshly and four unaffected targets are cached. This includes native/WebAssembly conformance and the historical differential evaluator. Formatting and build-integrated lint checks pass.

## Measurement

The [raw audit](resource.json) contains source and executable hashes, the baseline revision, exact arguments and fixtures, power state, preliminary experiments and every final sample. Optimized Bazel-built executables run serially on the current Apple M5 Max on AC power, after builds and tests complete. Three rounds alternate baseline/candidate order. Each lifecycle process records its first evaluation separately and then five fresh-instance samples. Compact execution and inspection use nine samples. Both sides use the same established 100 ms per-case symmetry warm-up and 25 samples.

The final matrix covers ten lifecycle workloads, three compact arithmetic controls, incremental inspection and twelve symmetry cases. All 26 gates pass: at most 5% regression, or 10% below one millisecond. State/event/work counts, canonical search steps and serialized report observations agree across all paired runs. Sixteen complete or paused command reports also compare byte-for-byte, including a 69.6 MB complete report. Allocation diagnostics run separately, preserve those observations and return to zero additional retained bytes after release.

| Workload | Baseline | Candidate | Baseline / candidate |
| --- | ---: | ---: | ---: |
| Six factors, full reporting | 563.027 ms | 525.333 ms | 1.07× |
| Six factors, complete lifecycle | 760.599 ms | 726.742 ms | 1.05× |
| Three factors, complete lifecycle | 147.540 ms | 142.257 ms | 1.04× |
| Nested expression, complete lifecycle | 228.440 ms | 217.860 ms | 1.05× |
| Compact six-factor execution | 65.172 ms | 65.797 ms | 0.99× |
| Ten-digit decimal addition | 201.180 ms | 193.095 ms | 1.04× |
| Five-digit decimal multiplication | 227.252 ms | 213.386 ms | 1.06× |

The reporting phase removes 1,529,395 allocation calls in the six-factor diagnostic, reducing 7,765,716 to 6,236,321, about 19.7%. It adds 4,626 successful reallocations as short lists spill. Requested lifecycle allocation traffic is approximately 2.219 GB before and 2.221 GB after, about 0.08% higher. Peak live requested storage remains approximately 515.8 MB. The wider combined resource index offsets the requested-byte savings of singleton storage; fewer calls and lookups, rather than lower peak memory, justify this change.

This is not a universal speedup. The 30-world ring control rises from 2.201 to 2.281 ms, within its 5% gate, and some smaller shared controls also become slightly slower. Complete arithmetic and reporting workloads remain within their gates. These are measurements on the current machine, not cross-machine speed guarantees.

The two component experiments are retained separately in the artifact. Singleton storage reduces allocation traffic; combining the resource and capture indexes removes another lookup pass and approximately 59,000 allocation calls during reporting, while widening the index entries. The final comparison validates the combined result.

Remaining peak storage is a separate problem. Retained canonical forms, report materialization and the serialization buffer coexist. The next investigation should attribute that retained storage before choosing a representation change. This increment makes no memory-peak or program-rewriting claim.

## Reproduction

```sh
bazel test -c opt //...
bazel run -c opt //:format -- --check
bazel run -c opt //benchmark:lifecycle -- expression '2*2*2*2*2*2' 2101 --sample 5
bazel run -c opt //benchmark:allocation -- expression '2*2*2*2*2*2' 2101 --sample 3
bazel run -c opt //benchmark:symmetry
```
