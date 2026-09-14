# Runtime

[Webbook](../index.html) · [Repository guide](../README.md)

- [Runtime](#runtime)
- [Coherence symmetry](#symmetry)
- [Performance](#performance)

<a id="runtime"></a>

## Runtime

The [configuration semantics](language.md#semantics) defines the shared contract of the native runtime and the bounded reference evaluator. Both follow one iterative rule mechanism: types are computation, and a derived witness can enable an application at its concrete source.

<a id="runtime-implementation-boundary"></a>

### Implementation boundary

| Component | Implemented behavior | Remaining boundary |
| --- | --- | --- |
| Rust frontend | Pest-backed native lowering and lossless structural parsing, source spans, diagnostics | Recovery, incremental editing, input-size budgets |
| Rust rule primitive | Literal multiset replacement and remainder broadcasting; exact whole nested values | Deliberately independent of configuration identity |
| Rust configuration runtime | Joint source projection, introduction reconciliation, nested captures, generated activation, canonical states, positive evidence closure, bounded CLI | Hard byte/time limits, arbitrary structure operations |
| JavaScript configuration reference | Joint evidence, source projection, multiple outputs, fresh introductions, nested frames, captured rule values and local activation, canonical configurations, positive evidence closure | Production execution, arbitrary partial structural matching |

The current laboratory is [reference.html](reference.html). The [causality report](research.html) preserves physical interpretation research. Retired executable laboratories link to the current plan.

<a id="runtime-rust-primitive"></a>

### Rust primitive

`rule::Rule::apply` accepts one concrete particle per input pattern, arranged in pattern order. It subtracts matched multiplicity, combines the remainder, and broadcasts that remainder to every literal output. It preserves duplicate values and leaves its inputs unchanged.

For `[A, B, C] → [D, E]` bound to `A.X`, `B.Y`, and `C.Z`, it returns `D.X.Y.Z` and `E.X.Y.Z`. This primitive does not identify shared inherited occurrences. A divergence/reunion round trip can therefore count carried context twice if a caller uses label multisets alone.

The generic concept parameter also permits `Rule<Rule<Concept>>`: exact matching can consume a whole rule value and replace it with another. Structural ordering of its stored representation is not behavioral equivalence or lexical alpha-equivalence. The separate Rust configuration runtime and JavaScript reference activate produced rules for finite ground constructors such as `{rule: {input, output}}`, with captured environment identity and ordinary application support. See [higher-order behavior](language.md#semantics-nested-rule-values).

<a id="runtime-execution-and-scheduling"></a>

### Execution and scheduling

Both configuration evaluators explore complete configurations and intern states, events, relative views, and support clauses. A new view can enable an application at an existing source. New evidence extends support without allocating a duplicate result.

Rust's `search::Search` retains particle-enumeration and gate progress. Per-target/frame/pattern caches deliver completed bindings incrementally to subscribers and reuse them across relative views. Adjacency indexes wake related work. Completed candidate searches release their temporary state.

`canonical::Search` refines graph colors using introduction sharing, captures, parent links, and lexical links, then advances through exact candidate orderings one step at a time. Refinement reduces avoidable symmetry; worst-case enumeration remains factorial. Completed normalization slots are reused.

Support starts from facts and advances positive premise counters. Every premise must be established before its conclusion is added. Cycles cannot bootstrap themselves. An unchanged runtime reuses this closure through `OnceLock`; advancing execution invalidates only the cached calculation, not previously established evidence.

The optional Rayon executor advances independent matching and canonicalization steps in parallel. The coordinator merges results in queue order and owns semantic interning and support updates. Worker completion order does not select a language interpretation. Rule availability remains read support, separate from consumed operands and return continuations.

Both evaluators require explicit positive premises. There are no default rules, absence queries, or conditional truth states. Rule availability remains part of the evidence for an application.

<a id="runtime-rust-execution"></a>

### Rust execution

`lowering::parse` produces `source::Program`; `runtime::Runtime::new` compiles that representation and initializes graph exploration. `run(steps, Some(limit))` advances the agenda, and `snapshot()` returns a serializable report with configurations, events, witness mappings, support, and pending-work counts. Calling `run` again resumes the same in-memory runtime. `parallel(&executor, steps, Some(limit))` uses an `executor::Executor` constructed with a positive worker count. A JSON report is not a reloadable checkpoint.

The CLI accepts [native source](language.md#syntax) or a structured JSON program. Run these commands from the repository root; Bazel supplies the toolchain and external crates:

```sh
bazel run //system:command -- run "$PWD/example/conjunction.wave"
bazel run //system:command -- run "$PWD/example/dynamic.wave" --json
bazel run //system:command -- run "$PWD/example/coherence.wave" --workers 4 --steps 12000 --records 1000000 --states 80 --cells 12 --frames 10 --coherences 4
bazel test //...
bazel run //:format -- --check
```

The CLI defaults to one worker and 12,000 agenda steps, with limits of 80 states, four coherences, 12 live and held occurrences per configuration, ten reachable frames, and one million retained records. Reports expose current `record` and observed `peak` counts. The record limit is soft: the coordinator checks it between batches, so one batch can exceed it. Record counts are not byte counts.

A paused report retains pending work; exhausting a budget does not supply a language-level fact. Candidate enumeration and ordering can resume, but source compilation, graph setup, closure normalization, and individual coordinator operations are not strictly preemptible. These budgets are not wall-clock or allocation guarantees.

Keep named crate roots, explicit modules, structured diagnostics, and the hermetic Bazel build. Measure matching visits, allocations, canonicalization, support growth, and event throughput before choosing production storage and indexing strategies.

Finite checks are acceptance evidence, not a proof of universal soundness or completeness. The accepted finite ground core is implemented. Arbitrary partial structural operations, hard resource isolation, live program edits remain outside its scope. [Platform verification](build.md#platform) distinguishes configured native CI from completed host and cross-build checks.

Native execution uses the [original grammar](language.md#syntax). Whole-rule generalization is documented [here](language.md#generalization); arbitrary structural substitution is not implemented. Negative premises are not supported in the language, runtime, or JSON.

<a id="symmetry"></a>

## Coherence symmetry

Implemented general canonicalization optimization. The rule application, source-inference, and ownership semantics are unchanged. This is not an arithmetic algorithm.

<a id="symmetry-swap-certificate"></a>

### Swap certificate

Two coherences may be interchanged by this optimization only when they have the same frame and the same resource description. Occurrences used outside a coherence, including held environments, retain their exact resource identity in that description. Resources confined to the coherence are compared by value, capture, and occurrence multiplicity, allowing anonymous renaming.

This certifies an automorphism fixing the surrounding state: swap the two coherences and consistently rename their private resources; shared resources, frame identities, and capture references remain fixed. Every certified swap preserves the state. The certificate is sufficient, not a complete automorphism algorithm. More complex symmetries remain in the ordinary search.

The ordering cursor enumerates only arrangements that retain increasing original indices within each certified class. It preserves the lexicographic order of that subset of the original exhaustive enumeration. Each omitted ordering has an earlier retained representative producing the same canonical state. This preserves the existing minimum and tie choice, including the renaming map used by provenance. It does not merge the coherences themselves or equate independent introductions with shared ones.

The optimization is attempted for configurations with at least four coherences and an unresolved refinement class. Smaller configurations use the ordinary permutation cursor because certificate construction can outweigh the saved work. This is an execution-cost threshold with identical meaning on either side, not a language rule or arithmetic special case. The iterator uses ordinary lexicographic permutation when there is no certified equivalence to exploit.

<a id="symmetry-reproduce"></a>

### Reproduce

```sh
bazel run -c opt //system:symmetry
bazel test //system:ordering //system:test
```

The benchmark measures canonicalization construction, incremental search, and extraction of a completed result. Input allocation happens before timing. Each case has one warm-up and 25 samples, with a 1,000-step search budget. Medians below are microseconds on the development host; incomplete rows time only the bounded prefix, not a completed normalization. [Raw measurements](symmetry.json).

| Sharing | Coherences | Search steps | Complete | Median µs |
| --- | ---: | ---: | --- | ---: |
| private | 2 | 5 | true | 4.54 |
| private | 4 | 3 | true | 3.42 |
| private | 8 | 3 | true | 5.33 |
| private | 30 | 3 | true | 15.58 |
| shared | 2 | 5 | true | 2.08 |
| shared | 4 | 3 | true | 3.08 |
| shared | 8 | 3 | true | 4.58 |
| shared | 30 | 3 | true | 11.67 |
| ring | 2 | 5 | true | 2.96 |
| ring | 4 | 49 | true | 28.54 |
| ring | 8 | 1000 | false | 750.17 |
| ring | 30 | 1000 | false | 1849.92 |

Eight independent, identically shaped coherences previously required 8! = 40,320 world orderings under the exhaustive cursor. The certificate reduces this case to one ordering. Thirty interchangeable coherences complete in three search steps; a 30-coherence sharing ring still suspends at 1,000 steps. Thus this removes one important source of factorial work without claiming to solve general graph symmetry.

<a id="symmetry-verification-and-limits"></a>

### Verification and limits

An independent permutation oracle exhaustively checks every three-class assignment for up to five indices, including partitioned orderings and exact enumeration order. Runtime tests retain all thirty resources for independent worlds, one resource shared across thirty worlds, and their unequal states. An additional 512 sharing graphs check invariance under world permutation and resource renaming. Existing tests compare complete semantic graphs and deterministic execution across worker counts.

A sequential before/after run of the 19 small reference benchmarks is recorded in [the comparison](symmetry-comparison.json). The median of the 19 per-program current/baseline latency ratios was approximately 1.10 in that sequential comparison. Those inputs are mostly too small or asymmetric to benefit substantially; timing variation and certificate overhead can dominate. This is not evidence of an overall application speedup. The targeted search-step reduction is the demonstrated improvement. Broader benchmarks and more efficient certificates remain work.

Parallel worlds remain distinct execution contexts. The existing worker executor can advance independent search and normalization jobs concurrently. This change improves each normalization job; it neither serializes the language nor establishes that arbitrary rules have disjoint effects.

<a id="performance"></a>

## Performance

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

<a id="performance-interpretation"></a>

### Interpretation

Scheduling overhead can outweigh independent computation on these small graphs. One worker remains the default; these measurements do not justify an unrestricted parallel speedup claim. Removing unnecessary language machinery simplifies this implementation, but the benchmark does not establish a universal performance bound.

Atoms and ground rule code are interned. States and views share immutable allocations, with hash indexes and exact comparison. Cached matcher cursors generate assignments incrementally; gates retain compatible prefixes and suppress duplicate arrivals. Completed search scratch storage is released and normalization slots are reused.

Canonicalization refines sharing, capture, lexical, and return edges before enumerating unresolved symmetries. The cursor is resumable, but worst-case CPU cost remains factorial. Regressions cover a large paused matching problem, a symmetric paused normalization, and progress of an independent small rule alongside a large search.

Positive evidence closure uses indexed premise counts. It agrees with a simple fixed-point oracle on 14,425 clause programs and traverses a 20,000-node chain without recursive dependency evaluation. An unchanged runtime reuses its support result.

<a id="performance-limits-and-reproduction"></a>

### Limits and reproduction

The soft record threshold counts retained graph, evidence, binding, request, matcher, normalization, and queued-work records. These records vary in size; the threshold is not a byte cap. The coordinator checks between batches, so a batch can overshoot. Paused work can resume with a larger budget.

Compilation, graph setup, individual renamings, closure import, and report construction are not strictly preemptible. Successor limits do not reject an already admitted initial program. Exact state sharing cannot terminate genuinely growing distinct configurations.

The browser's **Export reference fixtures** control recomputes all 20 positive cases. Rust compares canonical states and application edges for the 19 closed graphs and verifies suspension for the growing case. Expected graph changes require review; fixture generation is not permission to replace a semantic expectation automatically.

[Platform verification](build.md#platform) distinguishes native host testing, cross-compilation, and configured native CI jobs. Cross-build success alone does not prove native runtime behavior on another operating system.

<a id="performance-symmetry-reduction-follow-up"></a>

### Symmetry reduction follow-up

The [coherence symmetry optimization](#symmetry) now removes certified redundant world orderings while preserving resources, captures, and complete states. The earlier table above predates this change. A dedicated benchmark includes both improved interchangeable-world cases and unresolved sharing rings; the general worst case remains factorial.

<a id="performance-matching-and-composition-scheduling"></a>

### Matching and composition scheduling

The coordinator uses two FIFO queues. Matching, delivery, and application work can advance for at most 4096 removals before one pending transitive view-composition task runs. This changes exploration order without discarding inferred applications. Dedicated queue tests establish service under continuous foreground arrivals; the runtime suite checks chunking, record-budget resumption, parallel determinism, and semantic reference results. Small arithmetic remains expensive: the [retained-input product](arithmetic.md#product) reports its actual search size and unfinished exploration.

<a id="performance-immutable-programs-and-necessary-symbol-indexing"></a>

### Immutable programs and necessary-symbol indexing

Runtime instances share the compiled Program through Arc. A direct-path successor reuses the same compiled code instead of cloning its complete catalog. The program is immutable during execution; target compilation uses a separate mutable copy.

Each lexical scope indexes declarations by one required symbol, preferring the least frequent symbol in that scope. Empty-input patterns remain unconditional candidates. At inspection time, collect symbols from particles in the derived view's target, obtain candidate rule IDs, and sort them back into declaration order. A further check requires every pattern symbol to be present somewhere in that target before allocating a matching request. Live rule values receive the same presence filter.

Presence is only a necessary condition: full matching still checks multiplicity, distinct coherences, frame eligibility, and rule captures. No match can exist when a required symbol is absent from all target particles. Held frame resources are not pattern candidates in the matcher, so they are not included. Using the view target rather than its source preserves rule-derived abstraction. This optimization is internal to positive matching; it adds no absence premise to the language.

The indexed order is deterministic despite hash-set iteration. Scoped declarations, live code, empty patterns, inference, resumption, scheduling, and the semantic reference suite remain covered. An added regression verifies that 1000 irrelevant declarations do not change a source-inferred proof's reached states. The [arithmetic benchmark](arithmetic.md#word) records current measurements separately from earlier runtime versions.

## Query cost and browser execution

Obsidian separates `verdict()` from `report()`. Verdict queries use the canonical state index and cached positive support; they do not allocate node labels, event lists, or a full snapshot. Retargeting a search preserves the existing exploration. Bazel tests write that graph once and store small per-target verdicts beside it.

The webbook's evaluation diagram can now display either recorded native execution or a fresh run of this same Rust runtime compiled to WebAssembly. It draws configurations and runtime events directly, preserving alternative edges and cycles. The declaration diagram is a separate schematic; it does not pretend to determine an execution schedule. See the [browser and API contract](build.md#browser) for budgets, cancellation, generated bindings, and equivalence checks.
