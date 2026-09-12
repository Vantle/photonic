# Runtime implementation plan

The [interactive JavaScript plan](plan.html) is the main research document. The [semantic contract](semantics.md) defines the integrated reference behavior. This implementation sequence distinguishes tested behavior, accepted direction, and unresolved contracts. The finite dynamic JavaScript reference milestone is implemented. The production runtime is not yet complete or proven error-free.

## Accepted direction

Types are ordinary computation. A joint derivation can enable an application at its concrete source. That application consumes or transfers the concrete footprint; intermediate descriptions remain evidence. Coherences are independently evolving contexts. Decoherence requires a joint witness, preserves shared untouched remainder once, and retains independently produced equal results separately.

Exact meta matching treats a whole rule as one ordinary value. Applying a meta rule replaces its matched value in the successor, preserves unrelated remainder, and retains historical alternatives. No separate meta evaluator or global alias-equivalence operation is required.

A repeated canonical state shares computation. New evidence can add support and wake dependent applications. Genuine unbounded growth remains allowed; execution budgets suspend work. Conditional or contradictory reasoning remains represented according to explicit support semantics.

## Current implementation

| Component | Implemented | Boundary |
| --- | --- | --- |
| Rust frontend | Lossless structural parser, diagnostics, hermetic build | Executable source lowering is absent |
| Rust rule kernel | Generic exact multiset replacement; nested rule values use the same matcher | Literal broadcast remains provisional; no graph executor or dynamic activation |
| JavaScript configuration model | Source inference, joint decoherence, explicit outputs, remainder sharing, recursive body frames | Finite ground constructors, including generated executable rule values and captures |
| JavaScript matching gates | Incremental slot arrivals, retained compatible prefixes, duplicate suppression, out-of-order completion | Coherence slots with persistent per-target matching caches; no worker threads |
| JavaScript support | Conditional dependencies, negative cycles, independent justification, finite absence certificates | Immutable lexical declarations; generated local rule availability tracked through support |
| JavaScript canonicalization | Coherence order, anonymous identities, sharing, frame and continuation structure | Exhaustive symmetry enumeration; production optimization pending |

The reference passes 955 integrated checks, including 448 comparisons between incremental gates and exhaustive binding enumeration. The browser plan exposes the actual evaluator and a gate experiment. These checks support its finite contracts, not unrestricted language completeness.

## Matching gates

Each gate belongs to one joint witness configuration and frame. Matching provides slot candidates containing actual coherence and occurrence identities. The gate stores candidates by slot and compatible prefixes; a later slot can arrive before an earlier one. A prefix advances only when coherence positions are distinct and repeated equal patterns have a canonical assignment.

Completion emits a binding to the same source-projection and application operation used by direct matches. Rediscovered arrivals do not emit duplicate bindings. Event identity and support remain separate: another justification can support an existing application without producing another result. An incompatible history has a different gate and cannot contribute its arrivals.

This implements the useful part of the semaphore intuition. A scalar count cannot replace the binding record. The current reference is single-threaded, and particle-level literal matching still enumerates occurrences. Persistent per-target caches share binding work across views; adjacency indexes wake relevant paths. A production matcher still needs fine-grained resumable tasks and indexes across evolving configuration content. Synchronization primitives are implementation choices, not language semantics.

## Implemented reference decisions

The [closure contract](decision.md) records the accepted read/consume distinction and acceptance examples. Generated local rules, capture-aware identity, source projection, whole-rule abstraction, and conditional availability now execute in the same model. The dynamic examples in the browser are computed results, not illustrated predictions.

The structured representation distinguishes returning a whole rule value from entering a body. Exact rule-code identity normalizes orderless components and preserves capture edges in state identity. Dynamic programs conservatively keep absence open until supported evidence or finite completion resolves it; the unsafe fixed-rule shortcut is disabled.

## Remaining production work

Textual source lowering must map brackets and groups unambiguously to the executable structured representation. The Rust graph runtime must then implement the reference contract. Arbitrary structural extraction and code construction from unknown subterms require explicit operations beyond the current finite ground constructors. Production canonicalization, component-local absence certificates, parallel workers, resource accounting, and portability need conformance tests and measurement.

These boundaries limit implementation coverage; they do not justify adding a separate type engine or rejecting logically circular programs.

## Implementation sequence

| Step | Work | Acceptance |
| --- | --- | --- |
| 1. Dynamic rule reference — implemented | Local activation, read support, captures, replacement | A produced rule fires locally; replacement affects only its successor; competing rule/data histories cannot combine |
| 2. Rule identity and lowering | Ground value identity implemented; textual mapping remains | Whole nested values match without decomposition; different captures remain distinct; source reorderings follow the chosen semantics |
| 3. Dynamic support — conservative reference implemented | Keep dynamic queries open until evidence or finite closure; optimize certificates later | Generated rules invalidate dependent defaults; independent support survives; negative cycles remain conditional |
| 4. Indexed execution | Per-target binding cache and adjacency indexes implemented; scalable canonicalization and fine-grained suspension remain | Differential results match the finite reference; repeated evidence does not multiply events; finite work is not starved by growing alternatives |
| 5. Rust graph runtime | Port the settled reference contract into separate value, state, binding, event, and support concerns | End-to-end And, nested meta replacement and activation, many-to-many decoherence, scope, invalidation, and budget resumption |
| 6. Performance and portability | Benchmark representative programs and run native platform checks | Evidence for throughput, memory use, deterministic semantic results, and supported platforms |

Use external crates for established infrastructure where they fit. Keep Molten's binding, source projection, and support contracts explicit; do not delegate language semantics to a library whose behavior differs. Select runtime storage and matching dependencies after their required identities and update model are settled.

## Research documents

The [interactive plan](plan.html) contains the current examples, gate experiment, readiness table, and primary references. [Core](core.md), [binding](binding.md), [state](state.md), and [runtime](runtime.md) explain individual concerns. Earlier HTML experiments are marked historical and link to the current plan. Their superseded choices are not additional runtime requirements.
