# Performance

The benchmark measures fresh runtime construction, compilation into recursive program values, graph exploration, well-founded support, and construction of the inspection snapshot. JSON fixture decoding, input cloning, and worker-pool creation happen before timing. Source parsing, JSON report encoding, process startup, and build time are excluded. Each program receives one warm-up and 25 measured runs. Minimum, median, maximum, work, current records, and observed peak records are recorded in [the raw report](performance.json); timings are microseconds.

```sh
bazel run -c opt //system/molten/benchmark
bazel run -c opt //system/molten/benchmark -- --workers 4
```

Each run closes one of the 24 finite reference programs under a 12,000-step allowance and the default limits. Two intentionally growing programs are tested for suspension separately. The four-worker run must preserve the same semantic graph, work count, and record accounting. Runtime tests also compare full reports across one, two, and four workers, and across different pause sizes.

Re-measured after the structural-value extension on 2026-09-12 with hermetic Rust 1.98.1, Bazel optimized mode, Apple M5 Max, ARM64 macOS 26.6.2. This suite measures small-program latency and retained-record counts. It does not measure allocator bytes, resident memory, distributed throughput, or worst-case scalability.

Four workers are slower on these small programs because scheduling costs outweigh the independent computation. One worker remains the default. Parallel execution is available for workloads that justify it; these measurements do not support a general speedup claim. Resumable bookkeeping and exact graph refinement also add costs to small programs while preventing a large combinatorial task from monopolizing exploration.

| Program | States | Events | Work | Peak records | 1 worker µs | 4 workers µs |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Concrete And through two Boolean deductions | 14 | 17 | 1191 | 921 | 821.2 | 1672.6 |
| Two inputs broadcast their leftovers | 3 | 3 | 223 | 90 | 99.2 | 749.3 |
| One source supports a joint sibling match | 3 | 3 | 136 | 82 | 78.2 | 371.9 |
| Competing histories cannot supply a joint match | 3 | 2 | 86 | 87 | 48.6 | 244.4 |
| Explicit duplicate outputs remain distinct | 3 | 3 | 125 | 81 | 66.4 | 326.5 |
| Evidence-only co-results are not source leftovers | 4 | 3 | 115 | 102 | 67.3 | 334.7 |
| Independent branches compute independent equal results | 13 | 21 | 985 | 612 | 523.5 | 1544.0 |
| Arbitrarily nested body expressions | 9 | 9 | 501 | 480 | 276.9 | 1052.7 |
| A default defeated by another derivation | 2 | 1 | 43 | 42 | 20.7 | 105.8 |
| Independent support survives a defeated default | 2 | 2 | 49 | 55 | 30.0 | 114.8 |
| Self-dependent absence stays conditional | 2 | 1 | 48 | 44 | 20.2 | 105.1 |
| Mutual absence preserves unresolved alternatives | 3 | 2 | 100 | 101 | 40.6 | 167.0 |
| A finite cycle shares its configurations | 2 | 4 | 68 | 57 | 48.2 | 198.3 |
| Generated rule reads its concrete source | 4 | 4 | 103 | 97 | 75.4 | 288.0 |
| One source supplies rule and operand | 4 | 3 | 85 | 94 | 60.1 | 244.9 |
| Competing code and data cannot interact | 3 | 2 | 49 | 73 | 34.5 | 117.2 |
| A meta rule replaces a whole rule value | 6 | 6 | 284 | 214 | 138.5 | 709.2 |
| Rule activation respects coherence locality | 2 | 1 | 54 | 41 | 27.7 | 59.8 |
| A generated rule enables decoherence | 5 | 5 | 213 | 141 | 107.3 | 506.8 |
| Generated rules can defeat an absence condition | 6 | 7 | 269 | 227 | 135.0 | 515.6 |
| A whole rule acquires an abstraction | 8 | 8 | 336 | 329 | 181.2 | 760.6 |
| A generated rule opens a nested body | 6 | 7 | 206 | 191 | 138.6 | 488.8 |
| An escaped rule retains its local definition | 29 | 43 | 1952 | 2389 | 1308.7 | 2959.2 |
| Consuming code later does not erase earlier results | 4 | 3 | 159 | 86 | 58.3 | 471.9 |

## Cost controls and acceptance evidence

Symbols and structural rule code are interned. States and views use shared immutable allocations with exact equality and hash indexes. Matcher cursors generate particle assignments and gate extensions incrementally; target/frame/pattern caches deliver completed bindings to subscribed views. Completed search scratch storage is released. Normalization arena slots are reused.

Canonicalization refines the graph through resource sharing, capture, lexical, and return edges before enumerating unresolved symmetries. The enumeration cursor is resumable. A regression distinguishes a twelve-coherence asymmetric sharing chain within ten cursor steps; another pauses a thirty-coherence symmetric case after one hundred steps. Independently introduced initial coherences can be sorted and renamed directly because they have no inherited sharing; a hundred-coherence initializer avoids factorial enumeration.

A matcher regression pauses the selection of fifteen occurrences from thirty, a space of 155,117,520 combinations. A runtime regression lets an independent small rule produce its result while a large matching task remains open. These tests establish concrete progress and suspension behavior, not unrestricted real-time guarantees.

Support evaluation settles dependency components separately. More than 35,000 exhaustive/generated clause programs agree with the previous whole-graph alternating evaluator, including open queries and negative cycles. An unchanged runtime reuses the computed support report. A 20,000-node dependency chain tests iterative traversal without a recursive call stack.

## Resource boundaries

The `--records` threshold counts stored value structure and retained graph, query, support-clause, binding, request, matcher, normalization, and queued-work records. Reports expose current counts and an observed high-water mark after coordinator steps. Records have variable sizes: these counts are not bytes or an allocator-memory cap. The threshold is checked between coordinator batches; a batch or one bookkeeping operation can overshoot it. A paused runtime preserves its work and can resume after raising the threshold.

Canonicalization still has factorial worst-case CPU cost for unresolved symmetries. Initial program compilation, graph refinement setup, individual renamings, closure-environment normalization, and report construction are finite operations that are not strictly preemptible. Successor state/coherence/cell/frame limits do not reject the admitted initial program. Further optimization can improve these costs without adding language semantics.

## Reproducing conformance fixtures

Open [the runtime plan](plan.html) and select **Export reference fixtures**. The download recomputes all 26 graphs using the checked-in JavaScript evaluator. Compare it with `example/reference.json` before replacing that file. Rust tests compare canonical configurations and application edges with support for each closed graph; growing cases must remain suspended. A changed reference is a semantic contract change, not an automatically updated test expectation.

Native ARM64 macOS tests and x86-64 Linux/Windows cross-builds pass. [Six native CI jobs](../platform/README.md) are configured but have not run from this checkout. Platform configuration and cross-compilation alone do not establish native runtime support.

Structural matching, recursive capture graphs, and fuller record accounting increase the work represented by this baseline. These measurements supersede the earlier ground-only timings; they do not establish a speedup. The structural examples have separate correctness and suspension tests, rather than an extrapolated throughput claim.
