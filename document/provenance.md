# Compact provenance and graph storage

This phase follows [canonical refinement](refinement.md). It improves the runtime's representation and construction of provenance and refinement graphs. All arithmetic remains in the native Photonic program. Labels, captured frames, resource identity, event selection, and the work counter retain their existing semantics.

[Raw measurements](provenance.json) record the platform, input hash, executable hashes, protocol, and individual phase timings. The comparison uses the saved executable from the preceding cleanup phase and the new executable, alternating their order. Each invocation performs one warm-up and one measured evaluation; there are five measured evaluations per executable. Builds and tests finish before timing begins.

## Measurements

| Measurement | Before | After |
| --- | ---: | ---: |
| Median direct execution | 1.663 s | 1.171 s |
| Native events | 10,017 | 10,017 |
| Work tasks | 215,876 | 215,876 |

Execution time falls by 29.6% in the same-session comparison, a 1.42× speedup. This does not yet meet the sub-0.5-second target.

## Implementation and organization

`system/basis.rs` provides an immutable sorted set. Empty and singleton sets require no heap storage. Larger sets use a shared immutable slice, so cloning provenance does not copy its members. Construction sorts and deduplicates values and selects exactly one representation for each cardinality. Equality and hashing compare contents; shared allocation addresses have no semantic role.

`system/relation.rs` provides an immutable sorted map for completed provenance. Construction preserves the last value for duplicate keys, as the previous tree map did. Lookup uses binary search, and iteration retains key order. The interface exposes construction and reading without insertion into completed maps.

`system/application.rs` owns rule application and its mutable construction state. It appends provenance entries while building the result, then produces an immutable flow. Previously, application first built complete identity provenance and immediately discarded every world entry. It now constructs only the held-resource entries needed at that stage and adds surviving and produced worlds directly.

`system/flow.rs` owns identity, composition, projection, and renaming of completed flows. Singleton provenance stays inline through these operations. Larger output bases can share immutable storage across output tokens. Captured environments in runtime application identities also use shared ownership, avoiding deep copies when identities are retained by normalization and event records. These are shared values within execution; there is no new cache across events.

`system/graph.rs` stores typed adjacency in contiguous arrays with vertex offsets. Graph construction counts outgoing edges, computes offsets, then scatters connections into their vertex ranges. It preserves every edge, repeated edge, and edge kind. Refinement no longer allocates a separate adjacency vector for each vertex. `system/refinement.rs` constructs the graph, while `system/partition.rs` performs exact color refinement.

All these interfaces are internal. The runtime continues to prohibit unsafe code, and the build retains explicit Bazel source lists.

## Correctness and validation

All 99 Bazel test targets pass with `bazel test -c opt //... --test_output=errors`.

The compact-set and compact-map tests each enumerate 3,280 input sequences. They compare sorted contents against standard tree collections, including duplicate inputs, empty inputs, singleton construction, map replacement order, lookup, cloning, equality, and hashing of equivalent constructions. These tests establish the representation invariants used by flow equality and runtime deduplication.

The existing 6,017 graph/refinement cases also compare every adjacency row of the contiguous graph with its original representation before checking exact final color ordinals against full refinement. Existing runtime tests cover closed reference state sets, provenance, source inference, captured-resource mapping, shared identities, scheduler determinism, pause/resume, and native expression behavior. No test substitutes a host arithmetic answer for Photonic execution.

The benchmark still reports 10,017 events and 215,876 work tasks. This phase reduces execution time by changing provenance construction, copying, and graph storage inside those tasks. A smaller elapsed time does not imply a smaller task count.

## Experiments and research direction

A tested prototype stopped refinement once its requested leading vertices occupied distinct leading color classes. Exact comparisons against full refinement passed, but the workload did not improve measurably. The prototype was removed to keep the retained algorithm simpler.

The research direction for a larger improvement remains retaining useful computation across transitions. Differential dataflow demonstrates reuse through indexed changes and versioned iterative computation. Applying that model here would require explicit dependency and invalidation handling for resource renaming, captured environments, and provenance. The current changes do not implement differential dataflow or establish its potential speedup for Photonic. [McSherry, Murray, Isaacs, and Isard, Differential Dataflow](https://www.cidrdb.org/cidr2013/Papers/CIDR13_Paper111.pdf).

Canonicalization has separate obligations: color refinement alone does not replace exact canonical search. The refinement/individualization framework described by McKay and Piperno informs this boundary. Contiguous graph storage preserves the existing exact algorithm; no approximate graph signature replaces equality. [McKay and Piperno, Practical Graph Isomorphism, II](https://arxiv.org/abs/1301.1493).

Sub-0.5-second execution remains a target, not an achieved result or a proven impossibility. Runtime rebuilding and full graph analysis still occur between events. Further work should measure their remaining cost and compare any incremental implementation against complete recomputation.

## Reproduction

```sh
bazel build -c opt //mathematics/ternary:infix.program
output=$(bazel info -c opt bazel-bin)
bazel run -c opt //system:trace -- \
  "$output/mathematics/ternary/infix.assembly.json" \
  "$PWD/mathematics/ternary/result.particle" --sample 5
```

This measures initialization, direct execution, compact summary, and history release separately. Full JSON trace serialization is excluded. The reported timings are development-host measurements, not guarantees for other machines or programs.
