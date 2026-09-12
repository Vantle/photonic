# Runtime direction

The [configuration semantics](semantics.md) defines the current bounded reference, and the [implementation plan](plan.md) describes the path to a Rust runtime. Both follow one iterative rule mechanism: types are computation, and a derived witness can enable an application at its concrete source.

## Implementation boundary

| Component | Implemented behavior | Remaining boundary |
| --- | --- | --- |
| Rust frontend | Lossless structural parsing, source spans, diagnostics | Executable source lowering |
| Rust rule kernel | Literal multiset matching, replacement, and remainder broadcasting; exact whole nested rule-value matching | Coherence identity, graph execution, dynamic rule activation |
| JavaScript configuration reference | Joint evidence, source projection, multiple outputs, fresh introductions, nested frames, captured rule values and local activation, canonical configurations, conditional support | Production execution, general textual lowering, arbitrary partial structural matching |

The current laboratory is [plan.html](plan.html). The separate components in [program.html](program.html) and the [causality report](research.html) preserve earlier experiments; their former integration limits do not describe the current reference.

## Rust primitive

`rule::Rule::apply` accepts one concrete particle per input pattern, arranged in pattern order. It subtracts matched multiplicity, combines the remainder, and broadcasts that remainder to every literal output. It preserves duplicate values and leaves its inputs unchanged.

For `[A, B, C] → [D, E]` bound to `A.X`, `B.Y`, and `C.Z`, it returns `D.X.Y.Z` and `E.X.Y.Z`. This primitive does not identify shared inherited occurrences. A divergence/reunion round trip can therefore count carried context twice if a caller uses label multisets alone.

The generic concept parameter also permits `Rule<Rule<Concept>>`: exact matching can consume a whole rule value and replace it with another. Structural ordering of its stored representation is not behavioral equivalence or lexical alpha-equivalence. Rust does not activate produced rules. The JavaScript reference does so for finite ground constructors such as `{rule: {input, output}}`, with captured environment identity and ordinary application support. See [higher-order behavior](semantics.md#nested-rule-values).

## Execution and scheduling

The reference explores complete configurations and interns states, events, relative views, and support clauses. A new view can enable an application at an existing source. New evidence extends support without allocating a duplicate result. The reference caches match enumeration by target configuration, frame, and pattern. Slot-binding gates retain compatible partial matches within that joint configuration; cached bindings are reused across views. Adjacency indexes schedule related events, views, and queries. This is not a worker-thread executor or an unrestricted cross-configuration Rete network.

Rule availability and captured environments are premises of dynamic applications. The reference distinguishes their read support from consumed operands and keeps lexical capture separate from return destination. Production work still needs measured indexing, incremental support revisions, fair execution under broader workloads, and resource management.

The JavaScript revision comparison builds a new model with an added rule while preserving the earlier snapshot. It does not yet maintain arbitrary live program revisions incrementally. Its label-closure absence optimization is disabled whenever the compiled program contains rule values. Code availability remains part of support; unfinished queries cannot be certified absent merely because no current token matches.

## First Rust execution slice

Lower a specified source subset into the configuration representation, including ordinary abstraction rules and nested bodies. Compare direct and inferred Not/And applications against the reference before broadening syntax. Follow with joint divergence/decoherence, introduction reconciliation, and conditional support under the same event mechanism.

Keep named crate roots, explicit modules, structured diagnostics, and the hermetic Bazel build. Use external crates for standard functionality where their identity and lifetime models fit. Measure matching visits, allocations, canonicalization, support growth, and event throughput before choosing production storage and indexing strategies.

Finite reference checks are acceptance evidence, not a proof of universal soundness or completeness. The Rust runtime, executable textual lowering, arbitrary partial structural operations, and unrestricted absence guarantees remain work to complete.
