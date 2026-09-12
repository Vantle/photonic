# Runtime

The [configuration semantics](semantics.md) defines the current bounded reference, and the [implementation plan](plan.md) records the implemented Rust slice and remaining production work. Both follow one iterative rule mechanism: types are computation, and a derived witness can enable an application at its concrete source.

## Implementation boundary

| Component | Implemented behavior | Remaining boundary |
| --- | --- | --- |
| Rust frontend | Pest-backed native lowering and lossless structural parsing, source spans, diagnostics | Recovery, incremental editing, input-size budgets |
| Rust rule primitive | Literal multiset replacement and remainder broadcasting; exact whole nested values | Deliberately independent of configuration identity |
| Rust configuration runtime | Joint source projection, introduction reconciliation, nested captures, generated activation, canonical states, conditional support, bounded CLI | Production scaling, worker threads, arbitrary structure operations |
| JavaScript configuration reference | Joint evidence, source projection, multiple outputs, fresh introductions, nested frames, captured rule values and local activation, canonical configurations, conditional support | Production execution, arbitrary partial structural matching |

The current laboratory is [plan.html](plan.html). The separate components in [program.html](program.html) and the [causality report](research.html) preserve earlier experiments; their former integration limits do not describe the current reference.

## Rust primitive

`rule::Rule::apply` accepts one concrete particle per input pattern, arranged in pattern order. It subtracts matched multiplicity, combines the remainder, and broadcasts that remainder to every literal output. It preserves duplicate values and leaves its inputs unchanged.

For `[A, B, C] → [D, E]` bound to `A.X`, `B.Y`, and `C.Z`, it returns `D.X.Y.Z` and `E.X.Y.Z`. This primitive does not identify shared inherited occurrences. A divergence/reunion round trip can therefore count carried context twice if a caller uses label multisets alone.

The generic concept parameter also permits `Rule<Rule<Concept>>`: exact matching can consume a whole rule value and replace it with another. Structural ordering of its stored representation is not behavioral equivalence or lexical alpha-equivalence. The separate Rust configuration runtime and JavaScript reference activate produced rules for finite ground constructors such as `{rule: {input, output}}`, with captured environment identity and ordinary application support. See [higher-order behavior](semantics.md#nested-rule-values).

## Execution and scheduling

Both configuration evaluators explore complete configurations and intern states, events, relative views, and support clauses. A new view can enable an application at an existing source. New evidence extends support without allocating a duplicate result. Both cache matching by target configuration, frame, and pattern. JavaScript uses lazy slot-binding gates; Rust caches completed binding vectors. Bindings are reused across views of that joint configuration. Adjacency indexes schedule related events, views, and queries. This is not a worker-thread executor or an unrestricted cross-configuration Rete network.

Rule availability and captured environments are premises of dynamic applications. The reference distinguishes their read support from consumed operands and keeps lexical capture separate from return destination. Production work still needs measured indexing, incremental support revisions, fair execution under broader workloads, and resource management.

The JavaScript revision comparison builds a new model with an added rule while preserving the earlier snapshot. It does not yet maintain arbitrary live program revisions incrementally. Its label-closure absence optimization is disabled whenever the compiled program contains rule values. Code availability remains part of support; unfinished queries cannot be certified absent merely because no current token matches.

## Rust execution

`lowering::parse` produces `source::Program`; `runtime::Runtime::new` compiles that representation and initializes graph exploration. `run(steps, Some(limit))` advances the agenda, and `snapshot()` returns a serializable report with configurations, events, witness mappings, queries, support, and pending-work counts. Calling `run` again resumes the same in-memory runtime. A JSON report is not a reloadable checkpoint.

The CLI accepts [native source](syntax.md) or a structured JSON program. Run these commands from the repository root; Bazel supplies the toolchain and external crates:

```sh
bazel run //system/molten/command -- run "$PWD/example/conjunction.lava"
bazel run //system/molten/command -- run "$PWD/example/dynamic.lava" --json
bazel run //system/molten/command -- run "$PWD/example/coherence.lava" --steps 12000 --states 80 --cells 12 --frames 10 --coherences 4
bazel test //...
bazel run //:format -- --check
```

The default exploration budget is 12,000 agenda steps, with limits of 80 states, four coherences, 12 live and held occurrences per configuration, and ten reachable frames. A paused report retains conditional obligations; exhausting a budget does not establish absence. Individual matching and canonicalization tasks can still be expensive, so the step budget is not a wall-clock or allocation guarantee.

Keep named crate roots, explicit modules, structured diagnostics, and the hermetic Bazel build. Measure matching visits, allocations, canonicalization, support growth, and event throughput before choosing production storage and indexing strategies.

Finite checks are acceptance evidence, not a proof of universal soundness or completeness. Arbitrary partial structural operations, scalable canonicalization, parallel execution, and unrestricted absence guarantees remain outside the implemented fragment.
