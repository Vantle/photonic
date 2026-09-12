# Runtime implementation plan

The [interactive plan](plan.html) and [configuration contract](semantics.md) describe the current positive rule model. Native execution uses the [original grammar](syntax.md): concepts, dots, commas, contexts, groups, and whitespace. There are no variable sigils, named constructors, quotation operators, added punctuation, or built-in logical operations.

## Core decisions

Rules require explicit positive evidence. Types are ordinary derivations, and abstractions apply at concrete sources through the established witness-transfer convention. Whole rules are values that can be produced, matched, replaced, and activated locally. Meta rules and abstractions provide reuse without a separate evaluator.

Coherences preserve parallel evolution. Joint witnesses cannot mix competing histories. Shared untouched introductions reconcile once; independent results retain multiplicity. Exact canonical states share computation while new incoming evidence can enable additional events.

Negative premises are not a future feature. Their implementation and examples have been removed. A concept named Not is entirely program-defined, like every other concept.

## Delivered

| Component | Implementation | Boundary |
| --- | --- | --- |
| Frontend | One pest grammar, lossless source tree, executable lowering, structured diagnostics | Recovery, incremental editing, input-size budget |
| Matching | Incremental gates, distinct occurrence assignments, subscribed caches | Worst-case combinatorial search |
| Application | Joint source projection, consuming/read footprints, bodies, local generated code | Whole ground rule values; arbitrary field extraction is not implemented |
| State identity | Interning, graph refinement, resumable canonicalization | Unresolved symmetries can require factorial time |
| Evidence | Positive clauses, indexed premise counts, cached closure | No negative premises or default behavior |
| Execution | Fair agenda, resumable budgets, deterministic Rayon workers | Record thresholds are soft, not byte/time caps |
| Obsidian | Exact concrete reachability, witness report, resumable search | No independent portable certificate or universal theorem claim |
| Mathematics | Unary membership and independent-coherence addition | Equality, binding, proof objects, induction, and universal laws remain library work |

## Verification

The JavaScript suite passes 926 checks. Rust compares the 20 exported reference programs: 19 closed graphs and one growing program that suspends. Positive states and application edges agree with the previous positive fixtures; removal of absence behavior does not alter them.

Rust checks exact reports across worker counts and pause sizes, large matching/canonicalization suspension, progress of a small rule alongside a large search, capture identity, native syntax, CLI errors, and exact mathematical targets. Positive evidence closure agrees with a simple fixed-point oracle on 14,425 clause programs and handles a 20,000-node dependency chain iteratively.

```sh
bazel test //...
bazel run //:format -- --check
bazel run -c opt //system:command -- run "$PWD/example/conjunction.wave"
bazel run -c opt //system:command -- obsidian "$PWD/mathematics/natural/addition.wave" --target "$PWD/mathematics/natural/result.particle" --json
```

The [performance report](performance.md) records measured small-program costs. [Platform verification](../platform/README.md) separates completed native/cross-build checks from configured CI jobs.

## Remaining work

Establish encodings of unknown information, reusable hypotheses, binding, and induction through rule composition. Do not reinstate structural substitution under an implicit naming convention. An unproved encoding remains a research gap rather than a hidden language extension.

Build inspectable proof objects and a fixed positive checker library, then a portable certificate/replay format accounting for event and witness dependencies. Neither finite arithmetic examples nor a short path through a source-inferred graph proves a universal theorem.

Further runtime engineering can improve storage, indexing, input limits, and preemption while preserving observable events. Setup, individual graph operations, compilation, and reporting are not strictly preemptible. A finite benchmark is not a claim of unrestricted scalability or completeness.
