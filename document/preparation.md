# Immutable preparation reuse

This is the first CPU optimization stage after the [interior audit](interior.md), measured against `c2c26c5`. The [raw audit](preparation.json) records the source manifest, baseline harness, samples, power observations, controls, and validation. The [roadmap](roadmap.md) tracks the remaining stages separately.

## Implemented boundary

Particle matching now separates immutable candidate preparation from mutable combination selection. The immutable preparation owns the grouped token identities and input positions. Each search owns a compact, fixed-size selection buffer and its own completion state. Resetting or advancing one search cannot move another search.

A bounded preparation store reuses this immutable data across direct queries that reference the same compiled fragment, immutable world, and capture owner. Atom-only fragments normalize the irrelevant owner. Admission currently requires a fragment of at least eight terms and a world of at least 32 tokens. This is a structural cost gate, with no arithmetic-specific path. Small matching continues without cache lookup.

Keys retain weak identities for both fragment and world. Their control blocks prevent address recycling while a key exists, without retaining the world's contents. A changed world or a distinct capture produces a different key. Copy-on-write world mutation also separates the weak identity. Equal contents alone never establish cache identity. Dead identities are collected when admission needs space.

The store uses the existing shared 65,536-record cache budget. Admission failure falls back to ordinary preparation. Eviction drops the store's references; live searches continue with their immutable preparations and private cursors. Shared preparations remain conservatively included in each live search's logical accounting, and the cache separately charges the retained preparation plus its key/header. This does not claim byte-accurate allocator accounting. The focused reuse benchmark increases peak logical retention from 46 to 66 records.

Candidate lookup also returns occurrence IDs directly. All production consumers previously converted those IDs to current world ordinals and then back again. Empty-pattern candidates retain their prior ordinal order by sorting occurrence ranks. Plan fragments feed terms directly into posting lookup, avoiding an intermediate term array. The internal lookup API accepts an iterator; no frontend or public language semantics changed.

The reservation owner is shared by candidate and preparation caches. Combination advancement lives in `particle/group.rs`, cache identity in `preparation/key.rs`, cache policy in `preparation.rs`, and query integration in `plan.rs` and `joining/space.rs`.

## Preservation argument

1. Preparation uses the same grouping, exact term/capture predicates, sorted unique token IDs, and group order as the prior matcher.
2. The flattened private selection buffer replaces per-group mutable buffers without changing their combination sequence. Every emitted token remains an exact occurrence identity.
3. Cache hits require identical immutable objects and the relevant capture context. Mutation, replacement, and occurrence reuse cannot inherit an earlier world's preparation.
4. Cached preparation does not contain delivery position, proof evidence, executable reads, or mutable search progress. Those remain owned by their original consumer.
5. Candidate membership, candidate order, symmetry, and exact `Poll` sequences remain unchanged. No negative query is skipped speculatively; unsuccessful queries still retain their established traversal behavior.
6. Cache saturation and eviction retain the uncached enumeration path. Limits are unchanged and retained cache records remain charged.

## Paired native results

The primary audit uses three alternating rounds of nine samples per workload, with warmup, AC power checks, and no competing builds during timing. Commands use `bazel run -c opt` on an Apple M5 Max. The table reports pooled medians of 27 samples per side.

| Workload | Before, ms | After, ms | Ratio |
| --- | ---: | ---: | ---: |
| 256 queries sharing preparation of a 4,104-token world | 9.4097 | 0.2328 | 40.414× |
| Same queries with a private store each time | 9.4542 | 9.4386 | 1.002× |
| Stable prefix | 3.9194 | 3.5075 | 1.117× |
| Changing prefix | 4.2867 | 3.9213 | 1.093× |
| Six factors of two | 66.8477 | 65.3952 | 1.022× |
| Decimal 1234567890 + 9876543210 | 200.4906 | 199.7045 | 1.004× |
| Decimal 12345 × 67890 | 223.3282 | 222.0491 | 1.006× |
| Leaf reuse | 3.4459 | 3.4740 | 0.992× |
| 4,096-world candidate domain | 12.0144 | 12.1368 | 0.990× |
| Root fragment maintenance | 59.5846 | 61.9343 | 0.962× |
| Depth-two fragment maintenance | 241.0866 | 248.4517 | 0.970× |

The preparation result includes query construction, initial admission, enumeration, and query destruction. It excludes program/index construction. Each side emits 256 bindings in exactly 1,536 logical steps. The individual paired rounds show approximately 21×, 41×, and 41× improvement; the first current round has higher submillisecond latency. This is a repeated-preparation workload, not a claim of 40× faster arithmetic.

Additional 51-sample controls vary the extra world width independently. At 32 extra tokens, sharing improves about 2.04× while a private store costs about 4.4% more. At 256 extra tokens, sharing improves about 4.67× while a private store costs about 1.8% more. Below admission, the nine-token case improves about 1.07–1.17×. Those controls and all larger fragment, frontier, activation, and joining results are in the artifact.

All protected pooled workloads meet the predeclared 5% regression tolerance, or 10% for submillisecond cases. Small fragment workloads do not benefit uniformly; several are 2–4% slower. The initial exploratory large-domain slowdown did not persist in the final alternating audit. No known semantic discrepancy was accepted to achieve these numbers.

Reproduce the focused comparison with:

```sh
bazel run -c opt //benchmark:expression -- '2*2*2*2*2*2' 2101 --sample 9
bazel run -c opt //benchmark:dispatch -- --verify
```

This benchmark was removed after `0daf296` and still runs at that commit:

- [`preparation`](https://github.com/Vantle/photonic/blob/0daf296e1d126675ed445bfb7ed6674a379b1532/benchmark/preparation.rs) with `--sample 9`, `--private --sample 9` and `--width 32 --sample 51`

## Validation and remaining work

The optimized full suite passes all 106 targets, including native/WASM conformance. The debug kernel suite also passes. Four new tests exercise generated exact comparisons, independent interleaved cursors, capture separation, copy-on-write mutation, bounded admission, eviction while live searches remain, cross-plan reuse, and recycled occurrence slots.

The native subscription mutation harness retains identical complete observations in every paired round. Direct workloads retain identical event/work or binding/work counts. All 19 exhaustive fixtures retain identical state, event, work, record, and peak counts in ordinary and instrumented builds.

The post-change instrumented expression profile still records 66,492 search constructions and approximately 25 ms in subscription reconciliation, including roughly 10 ms in query preparation. These nested scopes must not be summed. The new cache targets expensive particle preparation; it does not remove the remaining subscription churn. Multilevel fragment maintenance, residual filtering, exhaustive fragment sharing, and bulk/parallel execution remain separate stages. GPU execution has not been introduced.
