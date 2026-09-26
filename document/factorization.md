# Incremental matching and reachability

The later [architecture audit](architecture.md) records localized domain updates and incremental grounded support. This audit continues the [shared matching implementation](implementation.md) against baseline `0969b6b`. The changes run through the ordinary native evaluator and proof matcher. Syntax, rule meaning, unordered state, synchronized firing, evidence, and browser defaults are unchanged. No arithmetic operation is recognized specially.

## Retained match factors

`factor.rs` owns a particle's reusable binding stream and its independent cursor. `joining.rs` retains those factors for surviving candidate worlds while applying removals and insertions to the domains. A change to another input can reset the outer product traversal without destroying a surviving factor's bindings. New candidates extend against those retained factors through the existing exact world-distinctness and symmetry constraints.

The representation remains factorized: it does not materialize the Cartesian product. Each factor contains concrete resource identities for one candidate world under one fixed capture context. It shares neither executable occurrences nor proof evidence. Removing a candidate drops its factor before inserting replacements, so numeric slot reuse cannot resurrect old answers. Consumer delivery and proof projection remain separate.

Known-impossible particle streams stay on direct enumeration. Admission starts after reuse for viable particle patterns of at least eight positions in multi-particle inputs. Retained factors have a 4,096-record query budget and a shared 65,536-record dispatch budget. A stream that exceeds either budget releases its acceleration data and continues ordinary enumeration. An incomplete prefix is never treated as a complete result or as evidence of absence.

Record pressure triggers eviction before the path evaluator suspends. If eviction occurs inside a replayed prefix, the factor reconstructs its exact enumeration cursor before releasing the cache. This preserves every subsequent `Poll`, binding, and logical work count. Reconstruction is bounded by the retained prefix. Discarded factors remain on direct enumeration until replaced, avoiding repeated allocation under pressure. These budgets measure logical records, not exact allocator bytes.

## Compiled matching

Pattern descriptors now support both source symbols and capture-specific terms. Exhaustive proof matching lazily compiles wider particle patterns once per query and reuses their position groups across candidate worlds. Small queries retain direct preparation. This puts compiled preparation in both evaluation paths without sharing live proof state.

For wide patterns against larger particles, preparation indexes demanded terms in one pass over the particle. It then constructs the same sorted, deduplicated resource candidates and occurrence-position mapping as the reference matcher. Small patterns avoid the hash table. Pattern-equivalence classes are compiled before enumeration, and gates accept borrowed patterns rather than cloning terms just to compare their shape.

Posting intersection chooses an existing strategy for small candidate lists, a monotone scan for similarly sized sorted postings, or a suffix search for skewed postings. Candidates retain their original order. Capture-specific terms, multiplicity checks, exact binding validation, and every cooperative enumeration step remain in force.

## Cyclic frame reachability

`reachability/network.rs` maintains incoming edges from reachable frame sources and a persistent membership map. Parent, lexical, and held-capture edges participate; world frames and token captures provide roots, alongside the permanent root frame.

For a changed root or topology, the algorithm invalidates the potentially affected old descendant region, stopping at independently rooted frames. It then restores the part reached by retained external sources, new roots, and new edges. Newly reached sources register their outgoing edges; lost sources retract theirs. An unrooted cycle is removed as a region and cannot justify its own restoration.

Incoming edges are retained only for reachable sources. This is necessary because reclamation clears unreachable frames and later reuses their slots. Keeping their old edges would make the index unsound after reuse. The index shares immutable storage when nothing relevant changes, and record pressure can discard it in favor of full traversal. Its cold, retained, and evicted states use one optional shared handle. An evicted index stays disabled across subsequent updates; the common case does not carry a separate policy flag beside its persistent data.

Measurements determine admission: at least 64 frame slots and four worlds per frame slot are required to build the retained index. It remains active down to 32 frame slots and two worlds per slot. This hysteresis avoids repeatedly switching representations near a threshold. These are implementation cost choices, not language limits. Output construction still scans retained membership, and a large affected region can require broad recomputation; this is not a constant-time dynamic reachability claim.

## Verification

The new tests exercise partial replay, cache saturation, shared budget accounting, eviction at multiple cursor positions, domain mutation, exact polling, slot reuse, capture-specific preparation, skewed postings, and interrupted full-program execution. Reachability is compared with complete traversal through randomized topology changes, reclamation, captured edges, and a targeted loss-and-reuse cycle case.

The earlier deep metaprogramming, recursive execution, competing evidence, unsupported support cycles, and browser conformance tests remain release gates. Structural generation of previously uncompiled rule shapes remains the separate [language proposal](dynamic.md).

Acceptance ran `bazel test --nocache_test_results //language:test` in debug mode and `bazel test -c opt --nocache_test_results //...`. All 106 targets passed, including 133 runtime unit tests, native program fixtures, and browser/WebAssembly conformance. The build also passed its Rust formatting, Clippy, and Starlark checks. Saved frontend expectations and production limits were unchanged. Linux and Windows execution, WebAssembly timing, and peak allocator bytes were not measured in this local run.

## Measurements

Measurements compare baseline `0969b6b314bea7c701541250d662f2435cefc6c6` with the native source digest in [factorization.json](factorization.json), on an Apple M5 Max running macOS 26.6.2 and Bazel 9.2.0. Both revisions were prebuilt, then run sequentially through `bazel run -c opt` with alternating revision order, at least 100 ms warmup, and seven samples. The laptop was on AC power, with sleep prevented during measurements. Tests and builds did not run concurrently with timing.

The table reports median execution time, excluding parsing, initialization, reporting, and release. Initialization samples are retained in the raw audit. Speedup is baseline divided by current time.

| Native path workload | Before (ms) | After (ms) | Speedup |
| --- | ---: | ---: | ---: |
| Six factors of two | 67.267 | 67.289 | 1.00× |
| 1234567890 + 9876543210 | 199.143 | 198.594 | 1.00× |
| 12345 × 67890 | 220.697 | 221.052 | 1.00× |
| 64 shared fragments, 256-token particle | 3.375 | 3.457 | 0.98× |
| 64 shared fragments, 4,096-token particle | 34.159 | 34.472 | 0.99× |
| 4,096-token deletion/update chain | 7.878 | 7.715 | 1.02× |
| 4,096-token unchanged-domain replay | 3.894 | 3.875 | 1.01× |
| Thirty factors of two | 3199.791 | 3186.034 | 1.00× |
| 256-term surviving factor, 1,000 changes | 3.865 | 3.555 | 1.09× |
| 128-term match, 8,192 noise tokens, 100 changes | 188.552 | 152.451 | 1.24× |

All measured path event and logical work counts agree. These results establish modest gains for retained factors and indexed wide matching, with essentially unchanged arithmetic. The protected path cases range from about 2.4% slower to 2.1% faster; those small differences do not establish a general speedup.

The following focused measurements isolate posting queries and reachability updates. Posting samples average ten queries. Reachability samples time 128 updates with 129 frame slots; state construction and initial index construction are excluded. The baseline uses the identical harness. These are component speedups, not whole-program speedups.

| Component workload | Before (µs) | After (µs) | Speedup |
| --- | ---: | ---: | ---: |
| Posting: 128 worlds, 1 term | 0.325 | 0.350 | 0.93× |
| Posting: 128 worlds, 2 terms | 0.617 | 0.508 | 1.21× |
| Posting: 1,024 worlds, 1 term | 3.163 | 2.763 | 1.14× |
| Posting: 1,024 worlds, 2 terms | 5.646 | 3.346 | 1.69× |
| Posting: 16,384 worlds, 1 term | 59.496 | 57.754 | 1.03× |
| Posting: 16,384 worlds, 2 terms | 141.246 | 51.796 | 2.73× |
| Reachability: 16 worlds, unchanged roots | 4.500 | 5.167 | 0.87× |
| Reachability: 16 worlds, changing roots | 43.875 | 44.917 | 0.98× |
| Reachability: 4,096 worlds, unchanged roots | 6.875 | 6.875 | 1.00× |
| Reachability: 4,096 worlds, changing roots | 858.875 | 123.833 | 6.94× |
| Reachability: 65,536 worlds, unchanged roots | 7.250 | 7.417 | 0.98× |
| Reachability: 65,536 worlds, changing roots | 13607.458 | 123.667 | 110.03× |

Across 19 exhaustive reference cases, the first paired run has a median per-case speedup of 0.98×, ranging from 0.94× to 1.06×. Three additional paired runs give median per-case ratios ranging from 0.98× to 1.07×, with an overall median of 1.00×. Each run uses 25 samples per case. Every state, event, work, retained-record, and peak-record count agrees with the baseline in all four comparisons. Exhaustive timing includes initialization, execution, snapshot, and release.

Small component overhead remains visible: the 16-world unchanged reachability case adds less than one microsecond across 128 updates in this sample set. Admission avoids imposing the full graph index on that case. The 110× result comes from avoiding repeated scans of 65,536 world roots after localized changes; it does not accelerate every phase of evaluation.

During acceptance, an intermediate representation caused a repeatable roughly 13% six-factor slowdown, including on AC power. Substituting the earlier reachability layer, shrinking its metadata, and moving its enclosing layout each recovered most of that loss. The accepted compact cache-state representation restored the protected expression timing without disabling incremental reachability. This is evidence of a representation-sensitive cost, not a claim that one specific hardware mechanism was proved.

Reproduce the focused cases through the native targets:

```sh
bazel run -c opt //benchmark:expression -- "2*2*2*2*2*2" 2101 --sample 7
bazel run -c opt //benchmark:arithmetic -- 1234567890 add 9876543210 --radix 10 --sample 7
bazel run -c opt //benchmark:runtime
```

These benchmarks were removed after `0daf296` and still run at that commit:

- [`factor`](https://github.com/Vantle/photonic/blob/0daf296e1d126675ed445bfb7ed6674a379b1532/benchmark/factor.rs) with `--width 256 --length 1000 --sample 7` and `--width 128 --noise 8192 --changing --length 100 --sample 7`
- [`posting`](https://github.com/Vantle/photonic/blob/0daf296e1d126675ed445bfb7ed6674a379b1532/benchmark/posting.rs)
- [`reachability`](https://github.com/Vantle/photonic/blob/0daf296e1d126675ed445bfb7ed6674a379b1532/benchmark/reachability.rs)

## Boundary

The production implementation now retains bounded leaf binding relations under deltas, shares compiled descriptors, uses adaptive intersections, and incrementally maintains cyclic frame reachability where its cost model admits it. It still recomputes outer join-product traversal after a relevant domain change. Arbitrary shared multi-particle subplans, retention of all surviving joined prefixes, cross-context rewrite-result reuse, and additional proof-flow sharing are not implemented by these changes.

Causal history reduction, generic instruction fusion, and broader parallel scheduling remain conditional research tracks. They need separate observation-preservation arguments and measurements; the existing inspectable histories and synchronization contract cannot be replaced merely to improve a benchmark. No result here establishes a universal orders-of-magnitude arithmetic improvement or proves the absence of all kernel bugs.
