# Performance

The benchmark measures runtime construction, ground code compilation, graph exploration, positive evidence closure, and snapshot construction. JSON decoding, input cloning, and worker-pool creation happen before timing. Source parsing, report encoding, process startup, and build time are excluded. Each program receives one warm-up and 25 measured runs. The [raw report](performance.json) records minimum, median, maximum, work, and retained records in microseconds.

```sh
bazel run -c opt //system:benchmark
bazel run -c opt //system:benchmark -- --workers 4
```

Measured after restoring the original grammar and removing negative-premise machinery on 2026-09-12 with hermetic Rust 1.98.1, Bazel optimized mode, Apple M5 Max, ARM64 macOS 26.6.2. These are small-program latency measurements, not allocator bytes, resident memory, distributed throughput, or worst-case scalability.

Each of the 19 finite reference programs closes under 12,000 steps and the default limits. One growing reference program is tested for suspension separately. One and four workers produce identical graph sizes, work, and record counts. Runtime tests also compare complete reports across worker counts and pause sizes.

| Program | States | Events | Work | Peak records | 1 worker µs | 4 workers µs |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Concrete And through two Boolean deductions | 14 | 17 | 594 | 582 | 358.5 | 816.0 |
| Two inputs broadcast their leftovers | 3 | 3 | 99 | 56 | 49.6 | 273.5 |
| One source supports a joint sibling match | 3 | 3 | 76 | 56 | 42.2 | 190.9 |
| Competing histories cannot supply a joint match | 3 | 2 | 59 | 63 | 25.1 | 152.8 |
| Explicit duplicate outputs remain distinct | 3 | 3 | 89 | 59 | 35.3 | 189.5 |
| Evidence-only co-results are not source leftovers | 4 | 3 | 67 | 72 | 35.1 | 170.1 |
| Independent branches compute independent equal results | 13 | 21 | 585 | 451 | 290.8 | 886.6 |
| Arbitrarily nested body expressions | 9 | 9 | 296 | 299 | 149.2 | 546.0 |
| A finite cycle shares its configurations | 2 | 4 | 56 | 47 | 28.0 | 128.3 |
| Generated rule reads its concrete source | 4 | 4 | 73 | 69 | 40.0 | 171.3 |
| One source supplies rule and operand | 4 | 3 | 59 | 64 | 30.7 | 135.2 |
| Competing code and data cannot interact | 3 | 2 | 37 | 50 | 18.4 | 74.7 |
| A meta rule replaces a whole rule value | 6 | 6 | 125 | 117 | 58.8 | 252.4 |
| Rule activation respects coherence locality | 2 | 1 | 38 | 31 | 15.5 | 44.1 |
| A generated rule enables decoherence | 5 | 5 | 118 | 88 | 55.2 | 251.7 |
| A whole rule acquires an abstraction | 8 | 8 | 200 | 206 | 89.8 | 390.9 |
| A generated rule opens a nested body | 6 | 7 | 145 | 132 | 73.2 | 274.9 |
| An escaped rule retains its local definition | 29 | 43 | 1334 | 1553 | 709.8 | 1559.0 |
| Consuming code later does not erase earlier results | 4 | 3 | 69 | 60 | 26.5 | 143.4 |

## Interpretation

Scheduling overhead can outweigh independent computation on these small graphs. One worker remains the default; these measurements do not justify an unrestricted parallel speedup claim. Removing unnecessary language machinery simplifies this implementation, but the benchmark does not establish a universal performance bound.

Atoms and ground rule code are interned. States and views share immutable allocations, with hash indexes and exact comparison. Cached matcher cursors generate assignments incrementally; gates retain compatible prefixes and suppress duplicate arrivals. Completed search scratch storage is released and normalization slots are reused.

Canonicalization refines sharing, capture, lexical, and return edges before enumerating unresolved symmetries. The cursor is resumable, but worst-case CPU cost remains factorial. Regressions cover a large paused matching problem, a symmetric paused normalization, and progress of an independent small rule alongside a large search.

Positive evidence closure uses indexed premise counts. It agrees with a simple fixed-point oracle on 14,425 clause programs and traverses a 20,000-node chain without recursive dependency evaluation. An unchanged runtime reuses its support result.

## Limits and reproduction

The soft record threshold counts retained graph, evidence, binding, request, matcher, normalization, and queued-work records. These records vary in size; the threshold is not a byte cap. The coordinator checks between batches, so a batch can overshoot. Paused work can resume with a larger budget.

Compilation, graph setup, individual renamings, closure import, and report construction are not strictly preemptible. Successor limits do not reject an already admitted initial program. Exact state sharing cannot terminate genuinely growing distinct configurations.

The browser's **Export reference fixtures** control recomputes all 20 positive cases. Rust compares canonical states and application edges for the 19 closed graphs and verifies suspension for the growing case. Expected graph changes require review; fixture generation is not permission to replace a semantic expectation automatically.

[Platform verification](../platform/README.md) distinguishes native host testing, cross-compilation, and configured native CI jobs. Cross-build success alone does not prove native runtime behavior on another operating system.

## Symmetry reduction follow-up

The [coherence symmetry optimization](symmetry.md) now removes certified redundant world orderings while preserving resources, captures, and complete states. The earlier table above predates this change. A dedicated benchmark includes both improved interchangeable-world cases and unresolved sharing rings; the general worst case remains factorial.

## Matching and composition scheduling

The coordinator uses two FIFO queues. Matching, delivery, and application work can advance for at most 4096 removals before one pending transitive view-composition task runs. This changes exploration order without discarding inferred applications. Dedicated queue tests establish service under continuous foreground arrivals; the runtime suite checks chunking, record-budget resumption, parallel determinism, and semantic reference results. Small arithmetic remains expensive: the [retained-input product](../mathematics/natural/product.md) reports its actual search size and unfinished exploration.

## Immutable programs and necessary-symbol indexing

Runtime instances share the compiled Program through Arc. A direct-path successor reuses the same compiled code instead of cloning its complete catalog. The program is immutable during execution; target compilation uses a separate mutable copy.

Each lexical scope indexes declarations by one required symbol, preferring the least frequent symbol in that scope. Empty-input patterns remain unconditional candidates. At inspection time, collect symbols from particles in the derived view's target, obtain candidate rule IDs, and sort them back into declaration order. A further check requires every pattern symbol to be present somewhere in that target before allocating a matching request. Live rule values receive the same presence filter.

Presence is only a necessary condition: full matching still checks multiplicity, distinct coherences, frame eligibility, and rule captures. No match can exist when a required symbol is absent from all target particles. Held frame resources are not pattern candidates in the matcher, so they are not included. Using the view target rather than its source preserves rule-derived abstraction. This optimization is internal to positive matching; it adds no absence premise to the language.

The indexed order is deterministic despite hash-set iteration. Scoped declarations, live code, empty patterns, inference, resumption, scheduling, and the semantic reference suite remain covered. An added regression verifies that 1000 irrelevant declarations do not change a source-inferred proof's reached states. The [arithmetic benchmark](../mathematics/arithmetic/README.md) records current measurements separately from earlier runtime versions.
