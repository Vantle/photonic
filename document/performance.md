# Performance

The benchmark measures fresh runtime construction, compilation into interned program values, graph exploration, well-founded support, and creation of the inspection snapshot. JSON fixture decoding and input cloning happen before timing; source parsing, JSON report encoding, process startup, and build time are excluded. Each program receives one warm-up and 25 measured runs. Results use microseconds, with minimum, median, and maximum recorded in [the raw report](performance.json).

```sh
bazel run -c opt //system/molten/benchmark
```

The benchmark uses the 24 closed programs from [reference.json](../example/reference.json), a 12,000-task allowance, and the default runtime limits. Every run must close. The two intentionally growing programs belong in suspension tests and are excluded from this latency benchmark. This is a small deterministic workload suite, not a throughput, memory, parallelism, or worst-case scalability benchmark.

Recorded on 2026-09-12 using the hermetic Rust 1.98.1 toolchain, Bazel optimized mode, Apple M5 Max, ARM64 macOS 26.6.2. Timings depend on host load, compiler, allocator, and hardware; they are an initial baseline, not a comparative performance claim.

| Program | Configurations | Events | Median µs |
| --- | ---: | ---: | ---: |
| Concrete And through two Boolean deductions | 14 | 17 | 326.2 |
| Two inputs broadcast their leftovers | 3 | 3 | 40.6 |
| One source supports a joint sibling match | 3 | 3 | 29.0 |
| Competing histories cannot supply a joint match | 3 | 2 | 23.8 |
| Explicit duplicate outputs remain distinct | 3 | 3 | 33.0 |
| Evidence-only co-results are not source leftovers | 4 | 3 | 24.0 |
| Independent branches compute independent equal results | 13 | 21 | 278.0 |
| Arbitrarily nested body expressions | 9 | 9 | 141.3 |
| A default defeated by another derivation | 2 | 1 | 12.7 |
| Independent support survives a defeated default | 2 | 2 | 18.9 |
| Self-dependent absence stays conditional | 2 | 1 | 12.2 |
| Mutual absence preserves unresolved alternatives | 3 | 2 | 22.1 |
| A finite cycle shares its configurations | 2 | 4 | 27.3 |
| Generated rule reads its concrete source | 4 | 4 | 36.5 |
| One source supplies rule and operand | 4 | 3 | 28.6 |
| Competing code and data cannot interact | 3 | 2 | 17.3 |
| A meta rule replaces a whole rule value | 6 | 6 | 55.2 |
| Rule activation respects coherence locality | 2 | 1 | 13.3 |
| A generated rule enables decoherence | 5 | 5 | 50.0 |
| Generated rules can defeat an absence condition | 6 | 7 | 64.6 |
| A whole rule acquires an abstraction | 8 | 8 | 88.8 |
| A generated rule opens a nested body | 6 | 7 | 69.5 |
| An escaped rule retains its local definition | 29 | 43 | 651.2 |
| Consuming code later does not erase earlier results | 4 | 3 | 25.3 |

## Implemented cost controls

Symbols and structural rule code are interned. States and witness views use shared immutable allocations, exact equality, and hash indexes. Matching results are cached per target, execution frame, and pattern. Reverse and forward dependency indexes wake relevant work; a FIFO agenda separates finite bookkeeping tasks without adding a semantic search/commit phase. Exact configuration canonicalization visits symmetry permutations incrementally, avoiding allocation of the entire permutation space. Equal incidence signatures use a deterministic introduction-ID tie-break so randomized hash iteration cannot change provenance maps or exploration work.

These optimizations preserve resource multiplicity and capture graphs. Equal fresh results remain independent introductions; broadcast references preserve shared introductions. State deduplication therefore cannot collapse genuinely different resources or competing alternatives.

## Scaling boundaries

Canonicalization still has factorial worst-case CPU cost for indistinguishable coherence or frame groups. One matching task eagerly enumerates finite candidate bindings, and one support report evaluates the whole clause set. The task budget is not a wall-clock or total-memory bound; a single large matching or canonicalization task can dominate execution. State, coherence, cell, and frame limits constrain generated successors, while views, bindings, clauses, and queued tasks have no separate memory budgets yet. The initial program is admitted before successor limits are enforced.

The next optimizations should refine canonical graph partitions, make matching and canonicalization suspend within a task, and maintain support by dependency component. Each change must retain full configuration, application-edge, and support agreement with the reference. Parallel workers require an independent correctness argument and measurements; the current runtime is single-threaded.

## Reproducing conformance fixtures

Open [the runtime plan](plan.html) and select **Export reference fixtures**. The download recomputes all 26 graphs using the checked-in JavaScript evaluator. Compare it with `example/reference.json` before replacing that file. Rust tests compare canonical configurations and application edges with support for each closed graph; growing cases must remain suspended. The browser export has been checked against the committed fixture data. A changed reference must be reviewed as a semantic contract change, not accepted merely to make a test pass.
