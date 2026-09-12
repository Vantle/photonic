# Runtime implementation plan

The [interactive JavaScript plan](plan.html) is the main research document. The [semantic contract](semantics.md) defines the integrated reference behavior. The accepted finite ground core is implemented end to end: native source, generated rule values, captured bodies, conditional support, resumable exploration, deterministic worker execution, and resource accounting. The accepted follow-on [structural-value extension](structure.md) is implemented in Rust; the JavaScript kernel remains the ground reference. This closes the core implementation plan without claiming universal correctness, termination, or unlimited scalability.

## Accepted direction

Types are ordinary computation. A joint derivation can enable an application at its concrete source. That application consumes or transfers the concrete footprint; intermediate descriptions remain evidence. Coherences are independently evolving contexts. Decoherence requires a joint witness, preserves shared untouched remainder once, and retains independently produced equal results separately.

Exact meta matching treats a whole rule as one ordinary value. Applying a meta rule replaces its matched value in the successor, preserves unrelated remainder, and retains historical alternatives. No separate meta evaluator or global alias-equivalence operation is required.

A repeated canonical state shares computation. New evidence can add support and wake dependent applications. Genuine unbounded growth remains allowed; execution budgets suspend work. Conditional or contradictory reasoning remains represented according to explicit support semantics.

## Current implementation

| Component | Implemented | Boundary |
| --- | --- | --- |
| Rust frontend | Pest-backed native lowering, lossless structural parser, diagnostics, hermetic build | Incremental editing, recovery, input-size budgets |
| Rust rule primitive | Generic exact multiset replacement; nested rule values use the same matcher | Independent of configuration identity |
| Rust configuration runtime | Joint inference, dynamic activation, captures, canonical state/event sharing, component-local support, resumable matching and normalization, refined canonicalization | Graph setup and closure normalization are not strictly preemptible |
| Rust CLI | Native or JSON program input; human or JSON report; worker selection; step and retained-record budgets | Soft record cap; in-memory resumption; no persisted checkpoint restore |
| Rust executor | Rayon workers advance independent searches; coordinator merges in queue order | Graph updates remain coordinated; throughput depends on workload |
| JavaScript configuration model | Source inference, joint decoherence, explicit outputs, remainder sharing, recursive body frames | Finite ground constructors, including generated executable rule values and captures |
| JavaScript matching gates | Incremental slot arrivals, retained compatible prefixes, duplicate suppression, out-of-order completion | Coherence slots with persistent per-target matching caches; no worker threads |
| JavaScript support | Conditional dependencies, negative cycles, independent justification, finite absence certificates | Immutable lexical declarations; generated local rule availability tracked through support |
| JavaScript canonicalization | Coherence order, anonymous identities, sharing, frame and continuation structure | Exhaustive symmetry enumeration; production optimization pending |

The integrated reference suite checks gates against exhaustive binding enumeration. Rust conformance covers all 26 reference fixtures. The 24 closed fixtures additionally compare exact reports for one, two, and four workers and continuation chunks of one, seven, and 32 steps. Support tests compare more than 35,000 programs against the whole-graph alternating evaluator. Other regressions cover independent initial-coherence normalization, oversized matching/normalization suspension, asymmetric sharing refinement, native lowering, deterministic provenance, and CLI diagnostics. These checks establish acceptance evidence for this finite contract.

## Matching gates

Each gate belongs to one joint witness configuration and frame. Matching provides slot candidates containing actual coherence and occurrence identities. The gate stores candidates by slot and compatible prefixes; a later slot can arrive before an earlier one. A prefix advances only when coherence positions are distinct and repeated equal patterns have a canonical assignment.

Repeated variables must agree across every slot, including captured value identity. Completion emits a binding to the same source-projection and application operation used by direct matches. Rediscovered arrivals do not emit duplicate bindings. Event identity and support remain separate: another justification can support an existing application without producing another result. An incompatible history has a different gate and cannot contribute its arrivals.

This implements the useful part of the semaphore intuition. A scalar count cannot replace the binding record. The JavaScript reference is single-threaded. Rust retains resumable particle and gate enumeration, delivers cached bindings incrementally to subscribers, and releases completed candidate state. Adjacency indexes wake relevant paths. Independent search steps can run on workers while the coordinator preserves queue order. Synchronization primitives are implementation choices, not language semantics.

## Implemented reference decisions

The [closure contract](decision.md) records the accepted read/consume distinction and acceptance examples. Generated local rules, capture-aware identity, source projection, whole-rule abstraction, and conditional availability now execute in the same model. The dynamic examples in the browser are computed results, not illustrated predictions.

The structured representation distinguishes returning a whole rule value from entering a body. Exact rule-code identity normalizes orderless components and preserves capture edges in state identity. Dynamic programs conservatively keep absence open until supported evidence or finite completion resolves it; the unsafe fixed-rule shortcut is disabled.

## Future boundaries

The native runtime now binds complete values inside named structures and rule fields, then constructs data and executable code with the same operation. Particle-rest capture and binder-syntax editing remain explicit future features. See [structural values](structure.md) for the exact boundary.

Graph refinement and resumable ordering reduce avoidable work, but exact canonicalization retains factorial worst cases. Source compilation, graph setup, and closure normalization are not strict preemption points. The retained-record budget can overshoot by a coordinator batch and is not a byte cap. Hard memory/time isolation and broader performance guarantees require further engineering and measurement.

Support SCCs optimize evaluation of existing clauses. They do not supply early component-local dynamic absence certificates or incremental edits to a closed program. Dynamic absence remains conservative until supported evidence or finite closure resolves it. Snapshots are reports, not restorable checkpoints.

Native CI is configured for all six platform triples. ARM64 macOS execution and x86-64 Linux/Windows cross-linking have passed locally; the other native jobs have not been run from this checkout. See [platform verification](../platform/README.md).

## Implementation sequence

| Step | Work | Acceptance |
| --- | --- | --- |
| 1. Dynamic rule reference — implemented | Local activation, read support, captures, replacement | A produced rule fires locally; replacement affects only its successor; competing rule/data histories cannot combine |
| 2. Rule identity and lowering — implemented | Ground value identity; explicit native code/body syntax | Whole nested values match without decomposition; different captures remain distinct; source reorderings follow the chosen semantics |
| 3. Dynamic support — conservative reference implemented | Keep dynamic queries open until evidence or finite closure; optimize certificates later | Generated rules invalidate dependent defaults; independent support survives; negative cycles remain conditional |
| 4. Indexed execution — implemented | Resumable matching, binding subscribers, graph refinement, streamed normalization, coordinator-ordered Rayon workers | Reference conformance, exact reports across worker/chunk choices, large-search suspension |
| 5. Rust graph runtime — implemented | Separate source, compilation, state, matching, flow, support, and execution modules | End-to-end And, nested meta replacement and activation, many-to-many decoherence, scope, invalidation, and budget resumption |
| 6. Resource accounting and portability — implemented within stated bounds | Soft retained-record accounting, temporary-state cleanup, [benchmarks](performance.md), six native CI jobs, cross-builds | Report current/peak records; native ARM64 macOS and Linux/Windows x86-64 cross-link checks; remaining native execution explicitly unverified |
| 7. Structural values — implemented | Shared data/code constructors, complete-value bindings, nested captures, alpha identity, seeded absence, resumable structural search | Native extraction, dynamic construction, scope preservation, source inference, and multi-coherence binding regressions |

Use external crates for established infrastructure where they fit. Keep Molten's binding, source projection, and support contracts explicit; do not delegate language semantics to a library whose behavior differs. IndexMap supplies stable indexed interning, Serde supplies data interchange, Rayon supplies worker scheduling, and the language-specific matching and projection remain explicit.

## Running and checking

```sh
bazel run //system/molten/command -- run "$PWD/example/conjunction.lava"
bazel run //system/molten/command -- run "$PWD/example/dynamic.lava" --workers 4 --records 1000000 --json
bazel test //...
bazel run //:format -- --check
```

Run from the repository root. Bazel supplies the Rust toolchain and external crates. [Frontend syntax](syntax.md) documents native input and the separate structural parser; [runtime](runtime.md) documents budgets and report semantics.

## Research documents

The [interactive plan](plan.html) contains the current examples, gate experiment, readiness table, and primary references. [Core](core.md), [binding](binding.md), [state](state.md), and [runtime](runtime.md) explain individual concerns. Earlier HTML experiments are marked historical and link to the current plan. Their superseded choices are not additional runtime requirements.
