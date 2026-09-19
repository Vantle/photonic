# Incremental evaluation

The runtime maintains typed hypergraph structure and candidate membership directly in Rust. It uses delta maintenance and shared partial results without introducing a database server, query language, native arithmetic shortcuts, or an execution order for Photonic. The frontend and browser resource defaults remain unchanged.

## Structural maintenance

`language/structure.rs` maintains world, frame, and resource vertices across structural comparisons. Immutable world identity and resource identity identify retained vertices; frame changes replace the relevant relationships. These identifiers are internal index keys and never contribute to the fingerprint. Vertex labels and incidence types match `language/incidence.rs`.

`language/propagation.rs` maintains four rounds of neighborhood refinement. Each vertex retains the sum, squared sum, and count of its neighbors' contributions at each round. Edge insertion and deletion adjust those accumulators. A changed color updates dependent contributions at the following round. If more than half the live vertex count is queued, the round switches to a full pass to avoid expensive dense propagation.

The resulting fingerprint is identical to full four-round recomputation. Eight-round filtering and exact canonicalization still handle remaining collisions. Repeated membership, distinct equal-valued resources, captures, lexical relationships, and the distinguished root remain represented. A state containing repeated references to the same world allocation uses full recomputation, preserving multiple world occurrences.

Only one mutable refinement cache is retained by a path search. Historical fingerprints remain immutable scalar values. The cache participates in record accounting and is discarded before optional cache retention can stop a search at its record budget.

## Matching and synchronization

The world index records removed and inserted sites. Cached selections retain every input domain, including domains belonging to an impossible conjunction. A selection update retracts removed membership and checks inserted worlds against its frame, symbol, and capture constraints. It preserves the selection allocation when membership is unchanged, including safe reuse of a site for a different world. Token enumeration still reads the current world, so unchanged candidate membership does not imply unchanged token bindings.

Capture-free input patterns share compiled terms and selections across lexical owners. Rules still retain their own owner, output, and read dependency. Capture-dependent patterns keep separate owner keys. Cached entries are evicted when no longer requested, and updates across a missing generation fall back to fresh preparation.

The gate matcher stores partial bindings as parent-linked arena entries. Extending a binding shares its existing prefix; only a complete result becomes a flat slot vector. Distinct-world constraints, synchronization, and symmetry checks retain their existing meaning. Matching order is an internal search optimization. The candidate-order regression test ensures that a reordered domain uses its corresponding pattern, and both direct and exhaustive evaluators reach the expected result.

The runtime still reconstructs rule scheduling after a transition. Candidate membership persists across transitions; gate prefixes are shared within a search. This implementation does not claim full cross-transition join maintenance or the complexity guarantees of a complete worst-case-optimal join engine.

## State layout

A transition computes reachable frames once for reclamation, then retains its cell count, next resource identifier, and reachable frame list alongside its fingerprint. Application, fingerprint aggregation, and limit checks reuse that layout. Historical states and complete transition provenance remain inspectable. State collections still share immutable worlds and frames; history has not been replaced by lossy summaries or checkpoints.

## Selection and verification

The design borrows incremental view maintenance principles from [Differential Dataflow](https://github.com/TimelyDataflow/differential-dataflow) and [DBSP](https://docs.feldera.com/vldb23.pdf). Shared intermediate representation follows the direction of [MAVIS](https://github.com/pkumod/MAVIS) and factorized query processing. These are algorithm references, not new dependencies.

A per-frame activation index was implemented and measured, then removed: maintaining its additional symbol counts and constructing local request lists was slower on the benchmark matrix. The selected implementation also avoids eagerly materializing every complete match.

Regression coverage compares incremental fingerprints with independent full recomputation across forward and reverse state sequences, renaming, repeated world references, and 1,024 generated mutations. Matching coverage includes candidate retraction, site reuse, capture reassignment, owner sharing, input multiplicity, unequal candidate cardinality, and independently enumerated multiway gate results. Existing exhaustive reference, provenance, pause/resume, budget, browser, and program tests remain enabled.

## Measurement

Measurements compare commit `b8010d2` with this implementation on the same Apple M5 Max running macOS 26.6.2. Every program runs through `bazel run -c opt`, with one warmup and three measured samples. Parsing, compilation, initialization, reporting, and release are outside the execution timer. Runs are sequential, without concurrent builds or tests. Raw samples are in [incremental.json](incremental.json).

| Program | Before | After | Speedup |
| --- | ---: | ---: | ---: |
| Six factors of two | 0.0809 s | 0.0648 s | 1.25× |
| Ten factors of two | 0.2308 s | 0.1850 s | 1.25× |
| Twenty factors of two | 1.3746 s | 1.0127 s | 1.36× |
| Thirty factors of two | 4.3365 s | 3.0387 s | 1.43× |
| Nested ternary expression | 0.0641 s | 0.0526 s | 1.22× |
| Ten-digit decimal addition | 0.2803 s | 0.1909 s | 1.47× |
| Five-digit decimal multiplication | 0.2901 s | 0.2185 s | 1.33× |
| Preparation reuse workload | 0.0162 s | 0.0148 s | 1.09× |

Every program reaches its expected result with the same successful transition count. Thirty factors still execute 151,476 transitions. This work improves implementation cost; it does not remove the separate browser state limit or establish an exponential or thousand-fold speedup.

Reproduce representative cases with:

```sh
bazel run -c opt //benchmark:expression -- '2*2*2*2*2*2' 2101 --sample 3
bazel run -c opt //benchmark:arithmetic -- 1234567890 add 9876543210 --radix 10 --sample 3
bazel run -c opt //benchmark:arithmetic -- 12345 multiply 67890 --radix 10 --sample 3
bazel run -c opt //benchmark:retention -- --width 200 --length 1000 --sample 3
```
