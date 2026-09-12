# Performance

The benchmark measures fresh runtime construction, compilation into interned program values, graph exploration, well-founded support, and construction of the inspection snapshot. JSON fixture decoding, input cloning, and worker-pool creation happen before timing. Source parsing, JSON report encoding, process startup, and build time are excluded. Each program receives one warm-up and 25 measured runs. Minimum, median, maximum, work, current records, and observed peak records are recorded in [the raw report](performance.json); timings are microseconds.

```sh
bazel run -c opt //system/molten/benchmark
bazel run -c opt //system/molten/benchmark -- --workers 4
```

Each run closes one of the 24 finite reference programs under a 12,000-step allowance and the default limits. Two intentionally growing programs are tested for suspension separately. The four-worker run must preserve the same semantic graph, work count, and record accounting. Runtime tests also compare full reports across one, two, and four workers, and across different pause sizes.

Recorded on 2026-09-12 with hermetic Rust 1.98.1, Bazel optimized mode, Apple M5 Max, ARM64 macOS 26.6.2. This suite measures small-program latency and retained-record counts. It does not measure allocator bytes, resident memory, distributed throughput, or worst-case scalability.

Four workers are slower on these small programs because scheduling costs outweigh the independent computation. One worker remains the default. Parallel execution is available for workloads that justify it; these measurements do not support a general speedup claim. Resumable bookkeeping and exact graph refinement also add costs to small programs while preventing a large combinatorial task from monopolizing exploration.

| Program | States | Events | Work | Peak records | 1 worker µs | 4 workers µs |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Concrete And through two Boolean deductions | 14 | 17 | 594 | 582 | 400.4 | 1067.1 |
| Two inputs broadcast their leftovers | 3 | 3 | 99 | 56 | 52.4 | 308.8 |
| One source supports a joint sibling match | 3 | 3 | 76 | 56 | 44.2 | 218.0 |
| Competing histories cannot supply a joint match | 3 | 2 | 59 | 63 | 30.9 | 162.8 |
| Explicit duplicate outputs remain distinct | 3 | 3 | 89 | 59 | 42.4 | 197.0 |
| Evidence-only co-results are not source leftovers | 4 | 3 | 67 | 72 | 44.2 | 173.5 |
| Independent branches compute independent equal results | 13 | 21 | 585 | 451 | 345.8 | 935.9 |
| Arbitrarily nested body expressions | 9 | 9 | 296 | 299 | 173.3 | 577.0 |
| A default defeated by another derivation | 2 | 1 | 31 | 32 | 15.1 | 73.7 |
| Independent support survives a defeated default | 2 | 2 | 37 | 41 | 20.7 | 81.1 |
| Self-dependent absence stays conditional | 2 | 1 | 36 | 34 | 15.3 | 78.4 |
| Mutual absence preserves unresolved alternatives | 3 | 2 | 73 | 76 | 29.0 | 129.6 |
| A finite cycle shares its configurations | 2 | 4 | 56 | 47 | 36.1 | 137.3 |
| Generated rule reads its concrete source | 4 | 4 | 73 | 69 | 49.6 | 174.3 |
| One source supplies rule and operand | 4 | 3 | 59 | 64 | 39.6 | 168.0 |
| Competing code and data cannot interact | 3 | 2 | 37 | 50 | 24.3 | 86.1 |
| A meta rule replaces a whole rule value | 6 | 6 | 125 | 117 | 75.5 | 275.9 |
| Rule activation respects coherence locality | 2 | 1 | 38 | 31 | 18.4 | 49.9 |
| A generated rule enables decoherence | 5 | 5 | 118 | 88 | 66.6 | 267.1 |
| Generated rules can defeat an absence condition | 6 | 7 | 179 | 167 | 88.0 | 339.6 |
| A whole rule acquires an abstraction | 8 | 8 | 200 | 206 | 108.2 | 414.1 |
| A generated rule opens a nested body | 6 | 7 | 145 | 132 | 90.8 | 292.9 |
| An escaped rule retains its local definition | 29 | 43 | 1334 | 1553 | 817.8 | 1740.8 |
| Consuming code later does not erase earlier results | 4 | 3 | 69 | 60 | 33.5 | 157.2 |

## Cost controls and acceptance evidence

Symbols and structural rule code are interned. States and views use shared immutable allocations with exact equality and hash indexes. Matcher cursors generate particle assignments and gate extensions incrementally; target/frame/pattern caches deliver completed bindings to subscribed views. Completed search scratch storage is released. Normalization arena slots are reused.

Canonicalization refines the graph through resource sharing, capture, lexical, and return edges before enumerating unresolved symmetries. The enumeration cursor is resumable. A regression distinguishes a twelve-coherence asymmetric sharing chain within ten cursor steps; another pauses a thirty-coherence symmetric case after one hundred steps. Independently introduced initial coherences can be sorted and renamed directly because they have no inherited sharing; a hundred-coherence initializer avoids factorial enumeration.

A matcher regression pauses the selection of fifteen occurrences from thirty, a space of 155,117,520 combinations. A runtime regression lets an independent small rule produce its result while a large matching task remains open. These tests establish concrete progress and suspension behavior, not unrestricted real-time guarantees.

Support evaluation settles dependency components separately. More than 35,000 exhaustive/generated clause programs agree with the previous whole-graph alternating evaluator, including open queries and negative cycles. An unchanged runtime reuses the computed support report. A 20,000-node dependency chain tests iterative traversal without a recursive call stack.

## Resource boundaries

The `--records` threshold counts retained graph, query, support-clause, binding, request, matcher, normalization, and queued-work records. Reports expose current counts and an observed high-water mark after coordinator steps. Records have variable sizes: these counts are not bytes or an allocator-memory cap. The threshold is checked between coordinator batches; a batch or one bookkeeping operation can overshoot it. A paused runtime preserves its work and can resume after raising the threshold.

Canonicalization still has factorial worst-case CPU cost for unresolved symmetries. Initial program compilation, graph refinement setup, individual renamings, closure-environment normalization, and report construction are finite operations that are not strictly preemptible. Successor state/coherence/cell/frame limits do not reject the admitted initial program. Further optimization can improve these costs without adding language semantics.

## Reproducing conformance fixtures

Open [the runtime plan](plan.html) and select **Export reference fixtures**. The download recomputes all 26 graphs using the checked-in JavaScript evaluator. Compare it with `example/reference.json` before replacing that file. Rust tests compare canonical configurations and application edges with support for each closed graph; growing cases must remain suspended. A changed reference is a semantic contract change, not an automatically updated test expectation.

Native ARM64 macOS tests and x86-64 Linux/Windows cross-builds pass. [Six native CI jobs](../platform/README.md) are configured but have not run from this checkout. Platform configuration and cross-compilation alone do not establish native runtime support.
