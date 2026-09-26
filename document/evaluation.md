# Evaluation runtime

The subsequent [incremental implementation](incremental.md) builds on this baseline and records its own measurements.

Photonic evaluation has hypergraph structure: a particle groups resource occurrences, shared resource identity connects particles, and a synchronized firing consumes and produces groups together. Flattening those groups into pairwise adjacency would lose information. The implementation uses a typed incidence graph with explicit world, frame, and resource vertices. Distinct edge types preserve containment, holding, capture, parent, and lexical relationships. Repeated membership remains repeated incidence, and the root frame has a distinct label.

`language/incidence.rs` supplies this representation to both exact canonicalization and structural filtering. An isomorphic renaming preserves every label and edge type. A hash match remains only a candidate for equality; the exact canonicalizer decides equality. Regression tests include shared versus independent resources, capture reassignment, reordered particles and frames, renamed resources, and a six-cycle versus two triangles that collide under neighborhood refinement but are not isomorphic.

## Structural filtering

The path runtime uses a cheap incremental fingerprint first. Only colliding buckets need a four-round incidence fingerprint. Remaining collisions receive an eight-round incidence fingerprint before exact comparison. Both refinements are computed lazily and cached on immutable history records. Refinement depth alone cannot establish equality: exact comparison remains necessary even after both filters agree.

Using eight rounds for every collision was slower than four rounds on the twenty-factor case. The staged design reserves that cost for the small subset that needs it. The underlying approach follows the partition-refinement foundations used in [nauty and Traces](https://pallini.di.uniroma1.it/Introduction.html).

## Incremental query preparation

`language/query.rs` interns identical rule inputs into shared plans. A dependency index connects each input symbol to the plans it can invalidate. Removing or inserting a particle invalidates plans referring to any symbol in that particle, including changes that preserve global symbol presence. Empty-particle patterns also depend on world occupancy.

Selections are keyed by plan, matching frame, and captured owner. Unchanged selections and immutable patterns survive transitions. Stable site identifiers are translated back to current world positions before matching. Rules with the same input can share a selection while keeping distinct outputs and read dependencies. Entries not requested in the current generation are evicted; retained cache storage participates in resource accounting.

Token enumeration, distinct-world constraints, and gate synchronization remain exact. Rule visibility and first-class rule occurrences are discovered from each current state. The cache stores candidate selections, not executable results or arithmetic answers. Its invalidation boundary is independent of the existing gate matcher, which remains available for future join-planning improvements.

This design applies the reuse and dependency principles of [incremental query maintenance](https://arxiv.org/abs/1303.5313) and shared materialized patterns. [MAVIS](https://mod.icst.pku.edu.cn/docs/2026-02/0e7ad13c50d94596b16b0879877549a3.pdf) and [Free Join](https://arxiv.org/abs/2301.10841) inform the longer-term choices of which subpatterns to materialize and how to join candidates. The current implementation does not claim their worst-case complexity guarantees.

## State storage and observability

Reachability is collected into an ordered vector instead of rebuilding a tree. Membership checks use binary search. Reclamation preserves already-cleared frames rather than cloning and clearing them again. Historical states remain immutable and inspectable, and transition provenance remains available.

The frontend language, gate semantics, arithmetic programs, and browser resource limits are unchanged. Twenty factors still exceed the browser's 32,768-state limit because the successful path contains 73,376 transitions. Faster execution does not remove that separate limit. The expression golden trace changes only its internal work count; the results and transition identifiers remain identical.

## Measurement

Measurements compare commit `27f1d5a` with this implementation on an Apple M5 Max running macOS 26.6.2. Each entry is the median of three native optimized execution samples after one warmup, run sequentially through Bazel. Parsing, compilation, initialization, reporting, and release are outside the execution timer. The baseline checkout received the same benchmark harness and observational counters, with its evaluation algorithm unchanged. Raw samples, counters, and limits are in [evaluation.json](evaluation.json).

| Program | Before | After | Speedup |
| --- | ---: | ---: | ---: |
| Six factors of two | 0.0834 s | 0.0794 s | 1.05× |
| Ten factors of two | 0.2649 s | 0.2319 s | 1.14× |
| Twenty factors of two | 2.6565 s | 1.3701 s | 1.94× |
| Thirty factors of two | 11.8833 s | 4.4601 s | 2.66× |
| `(12+21)*(22-10)+102`, ternary | 0.0710 s | 0.0651 s | 1.09× |
| `1234567890 + 9876543210`, decimal inputs | 0.6758 s | 0.2840 s | 2.38× |
| `12345 * 67890`, decimal inputs | 0.5343 s | 0.2927 s | 1.83× |
| Preparation reuse workload | 0.0648 s | 0.0168 s | 3.86× |

Every measured program reached the same expected result with the same successful transition count. On thirty factors, retained states requiring canonicalization fell from 30,092 to one, while 1,231,802 prepared selections were reused. The remaining runtime includes 151,476 actual transitions and their matching, state construction, and history costs. These measurements support multi-fold overall gains, not a claim of a thousand-fold runtime improvement.

The separate retention workload executes 1,000 transitions alongside 200 unchanged rules whose two-world input cannot match the single available resource world. It measures preparation reuse, not arithmetic throughput. It prepares 1,001 selections and reuses 199,999; its timing is included in the raw results.

## Reproduction

All benchmarks use the native Rust runtime through Bazel. The expression benchmark accepts ternary input and a nonnegative ternary expected result. Its readout program checks that result through ordinary Photonic rules.

```sh
bazel run -c opt //benchmark:expression -- '2*2*2*2*2*2' 2101 --sample 3
bazel run -c opt //benchmark:expression -- '2*2*2*2*2*2*2*2*2*2*2*2*2*2*2*2*2*2*2*2' 1222021101011 --sample 3
bazel run -c opt //benchmark:expression -- '(12+21)*(22-10)+102' 2122 --sample 3
bazel run -c opt //benchmark:arithmetic -- 1234567890 add 9876543210 --radix 10 --sample 3
bazel run -c opt //benchmark:retention -- --width 200 --length 1000 --sample 3
```

The [`trace`](https://github.com/Vantle/photonic/blob/0daf296e1d126675ed445bfb7ed6674a379b1532/benchmark/trace.rs) benchmark, removed after `0daf296`, also accepts `--state` for longer externally assembled programs. Increasing this benchmark limit does not change runtime or browser defaults.

`statistic.refinement` and `statistic.resolution` count retained states with cached four-round and eight-round fingerprints. `statistic.normalization` counts retained states with completed canonical forms; it excludes the separately stored goal and changes if callers subsequently inspect more history. `statistic.bucket` is the largest primary fingerprint bucket. `statistic.preparation` and `statistic.reuse` count freshly prepared and reused selections. Diagnostics are collected outside the execution timer.
