# Runtime

The [configuration semantics](semantics.md) defines the current bounded reference, and the [implementation plan](plan.md) records closure of the accepted core and future boundaries. Both follow one iterative rule mechanism: types are computation, and a derived witness can enable an application at its concrete source.

## Implementation boundary

| Component | Implemented behavior | Remaining boundary |
| --- | --- | --- |
| Rust frontend | Pest-backed native lowering and lossless structural parsing, source spans, diagnostics | Recovery, incremental editing, input-size budgets |
| Rust rule primitive | Literal multiset replacement and remainder broadcasting; exact whole nested values | Deliberately independent of configuration identity |
| Rust configuration runtime | Joint source projection, introduction reconciliation, nested captures, generated activation, canonical states, conditional support, bounded CLI | Hard byte/time limits, particle-rest capture, binder-syntax editing |
| JavaScript configuration reference | Joint evidence, source projection, multiple outputs, fresh introductions, nested frames, captured rule values and local activation, canonical configurations, conditional support | Production execution, arbitrary partial structural matching |

The current laboratory is [plan.html](plan.html). The separate components in [program.html](program.html) and the [causality report](research.html) preserve earlier experiments; their former integration limits do not describe the current reference.

## Rust primitive

`rule::Rule::apply` accepts one concrete particle per input pattern, arranged in pattern order. It subtracts matched multiplicity, combines the remainder, and broadcasts that remainder to every literal output. It preserves duplicate values and leaves its inputs unchanged.

For `[A, B, C] → [D, E]` bound to `A.X`, `B.Y`, and `C.Z`, it returns `D.X.Y.Z` and `E.X.Y.Z`. This primitive does not identify shared inherited occurrences. A divergence/reunion round trip can therefore count carried context twice if a caller uses label multisets alone.

The generic concept parameter also permits `Rule<Rule<Concept>>`: exact matching can consume a whole rule value and replace it with another. Structural ordering of its stored representation is not behavioral equivalence or lexical alpha-equivalence. The separate Rust configuration runtime and JavaScript reference activate produced rules for finite ground constructors such as `{rule: {input, output}}`, with captured environment identity and ordinary application support. See [higher-order behavior](semantics.md#nested-rule-values).

## Execution and scheduling

Both configuration evaluators explore complete configurations and intern states, events, relative views, and support clauses. A new view can enable an application at an existing source. New evidence extends support without allocating a duplicate result.

Rust's `search::Search` retains particle-enumeration and gate progress. Per-target/frame/pattern caches deliver completed bindings incrementally to subscribers and reuse them across relative views. Adjacency indexes wake related work. Completed candidate searches release their temporary state.

`canonical::Search` refines graph colors using introduction sharing, captures, parent links, and lexical links, then advances through exact candidate orderings one step at a time. Refinement reduces avoidable symmetry; worst-case enumeration remains factorial. Completed normalization slots are reused.

Support evaluates dependency strongly connected components in dependency order, keeping uncertainty across component boundaries. An unchanged runtime reuses its support result through `OnceLock`; advancing execution invalidates that cache. This does not implement incremental program editing or early dynamic absence certification.

The optional Rayon executor advances independent matching and canonicalization steps in parallel. The coordinator merges results in queue order and owns semantic interning and support updates. Worker completion order does not select a language interpretation. Rule availability remains read support, separate from consumed operands and return continuations.

The JavaScript revision comparison builds a new model with an added rule while preserving the earlier snapshot. It does not yet maintain arbitrary live program revisions incrementally. Its label-closure absence optimization is disabled whenever the compiled program contains rule values. Code availability remains part of support; unfinished queries cannot be certified absent merely because no current token matches.

## Rust execution

`lowering::parse` produces `source::Program`; `runtime::Runtime::new` compiles that representation and initializes graph exploration. `run(steps, Some(limit))` advances the agenda, and `snapshot()` returns a serializable report with configurations, events, witness mappings, queries, support, and pending-work counts. Calling `run` again resumes the same in-memory runtime. `parallel(&executor, steps, Some(limit))` uses an `executor::Executor` constructed with a positive worker count. A JSON report is not a reloadable checkpoint.

The CLI accepts [native source](syntax.md) or a structured JSON program. Run these commands from the repository root; Bazel supplies the toolchain and external crates:

```sh
bazel run //system/molten/command -- run "$PWD/example/conjunction.lava"
bazel run //system/molten/command -- run "$PWD/example/dynamic.lava" --json
bazel run //system/molten/command -- run "$PWD/example/coherence.lava" --workers 4 --steps 12000 --records 1000000 --states 80 --cells 12 --frames 10 --coherences 4
bazel test //...
bazel run //:format -- --check
```

The CLI defaults to one worker and 12,000 agenda steps, with limits of 80 states, four coherences, 12 live and held occurrences per configuration, ten reachable frames, and one million retained records. Reports expose current `record` and observed `peak` counts. The record limit is soft: the coordinator checks it between batches, so one batch can exceed it. Record counts are not byte counts.

A paused report retains conditional obligations; exhausting a budget does not establish absence. Candidate enumeration and ordering can resume, but source compilation, graph setup, closure normalization, and individual coordinator operations are not strictly preemptible. These budgets are not wall-clock or allocation guarantees.

Keep named crate roots, explicit modules, structured diagnostics, and the hermetic Bazel build. Measure matching visits, allocations, canonicalization, support growth, and event throughput before choosing production storage and indexing strategies.

Finite checks are acceptance evidence, not a proof of universal soundness or completeness. The accepted finite ground core is implemented. Rust extends the core with [structural matching and construction](structure.md). Particle-rest capture, binder-syntax editing, hard resource isolation, live program edits, and unrestricted absence guarantees remain outside its scope. [Platform verification](../platform/README.md) distinguishes configured native CI from completed host and cross-build checks.

## Obsidian

[Obsidian](../mathematics/obsidian.md) checks exact concrete configuration reachability through the existing Rust evaluator. It distinguishes supported witnesses, finite closed unreachability, and unknown results from unfinished or conditional exploration. JSON reports retain the supplied program, target, and execution evidence. They are not yet portable proof certificates.
