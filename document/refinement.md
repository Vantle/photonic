# Faster canonical refinement

The subsequent [compact provenance phase](provenance.md) reduces execution time further and records the next set of implementation changes.

This phase reduces execution latency after [indexed matching](indexing.md). The native evaluator, particle labels, matching semantics, and work counter are unchanged. Measurements below compare the indexed executable with the refined runtime on the same expression and budgets. [Raw measurements](refinement.json) include the input hash, platform, complete samples, and command arguments.

## Measurements

| Measurement | Indexed baseline | Refined runtime |
| --- | ---: | ---: |
| Median core execution | 2.280 s | 1.683 s |
| Median compact CLI | 2.252 s | 1.669 s |
| Native events | 10,017 | 10,017 |
| Work tasks | 215,876 | 215,876 |

Core execution falls by 26.2%; compact CLI time falls by 25.9%. CLI samples alternate executable order after one warm-up each. Phase measurements use one warm-up and five samples per executable, baseline followed by current. The baseline is the saved indexed executable from the preceding phase; same-session measurements differ slightly from its earlier 2.232-second execution median. Compact output is byte-for-byte equal between these executable samples.

## Implementation

Graph construction lives in `system/refinement.rs`; the graph-independent classification and refinement algorithm lives in `system/partition.rs`, with its oracle tests in `system/test/partition.rs`. This separates state representation from the partition algorithm without adding a public interface.

Canonicalization distinguishes graph vertices by repeatedly classifying their old color and their sorted, typed neighbor colors. The old implementation rebuilt every vertex signature and sorted the entire graph every round.

The new implementation maintains ordered ranges of vertices with equal colors. Because the old color is the first signature component, different classes can never merge or change relative order. Sorting independently inside each range therefore produces the same class order as sorting the complete graph. A singleton range cannot split and needs no neighbor signature. The algorithm stops when no range splits; it still returns the original algorithm's exact color ordinals.

Neighbor signatures occupy slices of one reusable buffer. Partition boundaries and color storage are also reused between rounds. This reduces allocation and repeated sorting without using approximate signatures or hash equality as a substitute for graph equivalence.

Resource lookup during graph construction uses hash maps. Map iteration order cannot affect refinement because neighbor signatures are sorted. Resource identity, edge kind, repeated edges, and captured frames all remain represented.

Frame ordering gathers occupied positions and captures in one traversal for each selected world ordering. The keys have the same values as before, but each frame no longer rescans the complete world collection. Reachability marks frames when enqueued, preventing repeated visits through shared captures. It still returns the same sorted set.

Normalization transfers ownership of the applied state to canonical search. Only provenance flow remains alongside that search; an extra complete state clone is unnecessary. Flow renaming consumes the canonical mapping as before.

## Validation

`bazel test -c opt //... --test_output=errors` passes all 99 test targets. The runtime suite includes exact refinement comparisons against an independent implementation of the original full-signature algorithm:

- All 512 directed graphs on three vertices, with all eight binary label assignments.
- Typed chains of length 2 through 64, including repeated edges and many refinement rounds.
- 1,856 deterministic generated directed graphs with four edge kinds and 4 through 32 vertices.
- Empty and singleton graphs.

These 6,017 comparisons check exact color ordinals, not just equivalent partitions. Existing tests also cover canonical resource graphs, captures, source inference, deterministic scheduling, pause/resume, native expression width and nesting, and browser expression playback. Finite test coverage supplements the ordering argument; it is not a universal proof of the runtime.

## Experiments and remaining work

A prototype recomputed only classes adjacent to a recently split class. Its reverse-edge bookkeeping did not improve this workload, so it was removed. Sorting borrowed resource incidence entries also showed no improvement and was removed. The retained implementation uses the simpler ordered partition algorithm.

No new parallel execution is introduced. This direct path still chooses one native event at a time. Matching work remains 215,876 tasks and the expression still performs 10,017 events. Graph construction, refinement, and allocation occur inside those tasks, so reducing their CPU cost does not reduce the task counter.

The next larger opportunity is reuse across events: update graph and matching information only where a transition changes it. That requires explicit handling of resource renaming, captured environments, provenance, and invalidation, with comparison against complete recomputation. It is an engineering direction, not a demonstrated speedup. A compact representation of identity provenance is another candidate for measurement. Neither proposal requires weakening Photonic semantics or implementing arithmetic in the host runtime.

These changes do not establish a performance floor. They also do not demonstrate execution in tens of milliseconds. That target needs further measured implementation work.

## Reproduction

Build and run the in-process benchmark with Bazel as the only required build dependency:

```sh
bazel build -c opt //mathematics/ternary:infix.program
output=$(bazel info -c opt bazel-bin)
bazel run -c opt //system:trace -- \
  "$output/mathematics/ternary/infix.assembly.json" \
  "$PWD/mathematics/ternary/result.particle" --sample 5
```

The benchmark performs one warm-up, then five measured evaluations. It separates initialization, execution, compact summary, and history release. Full JSON trace rendering is excluded. Run timing comparisons after builds and tests finish, with competing workloads stopped. Results here are measurements on one development machine, not cross-platform latency guarantees.
