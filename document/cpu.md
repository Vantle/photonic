# CPU reuse and execution checkpoint

This checkpoint extends the [subscription audit](reconciliation.md) and [binding-payload audit](segment.md). It changes implementation, storage and admission policy. It adds no syntax, arithmetic shortcut, world execution order or GPU path. Numerical results below are focused measurements; they are not multiplicative or universal end-to-end gains. Final integrated acceptance is recorded separately in the raw audit and validation section.

## Ownership

| Component | Owns | Does not share |
| --- | --- | --- |
| `dispatch/registry.rs` | Ordered subscriptions within each frame | Consumer progress |
| `dispatch/context.rs` | Optional reverse lexical adjacency | Language execution order |
| `joining/binding.rs` | Immutable selection payloads and parent links | Cursors, live dependency registrations |
| `gate/store.rs` | Bounded decisions for abstract partial world assignments | Tokens, query agendas, proof evidence |
| `gate/sharing.rs` | Per-query handles into the decision store | Another query's enumeration position |
| `flow/composition.rs` | The exact composition operation | Fresh rewrite identity |
| `flow/store.rs` | Bounded reusable unions for one exact immutable parent | Flows from another source identity |
| `work.rs` | CPU batch admission and one-step work execution | Publication or logical-budget decisions |

The runtime still coordinates synchronization, limits and publication. Matching emits exact bindings; rewrite and proof projection preserve source-specific evidence. New cache ownership includes accounting and eviction. The language kernel remains `forbid(unsafe_code)`.

## Exhaustive assignment reuse

A gate extends a partial assignment only when the new world is distinct from all prior worlds and respects the existing ordering between equal input patterns. The truth of this predicate depends only on the sequence of world numbers and pattern-equivalence classes. It does not depend on token identity, frame capture, state contents or the meaning of a class number in another query.

The cache interns that abstract sequence incrementally. Its key is the canonical parent identity and the next `(class, world)` constraint. Success carries a new canonical identity; failure is retained as an exact negative decision. Gates still reconstruct bindings from their private token-bearing prefix arenas. The existing exact pattern classifier supplies the class relation, including capture-sensitive equality.

This permits sharing across queries and even across snapshots without identifying their live occurrences: only a pure integer predicate is reused. Two class-number sequences can share a decision only when the equality comparisons along that entire keyed sequence are identical. No cached token or proof crosses the boundary.

Admission starts at 512 inputs. A 128-input cold control regressed in the experiment and is excluded. Focused repeated 512-input and 2,048-input gates improve approximately 4.0× and 10.9×, including construction and exact binding reconstruction. Cold large controls remain competitive. Small gates retain private traversal. The store is bounded independently of preparation storage and is included in the runtime's aggregate record count.

Eviction clears the cache and advances its generation. Old handles cannot refer to new entries. A stale, saturated or unadmitted parent falls back to the original scan; it is never mistaken for an empty prefix. Runtime record pressure also drops per-query cache handles. Every attempted extension still consumes exactly one gate step. This shares compatibility work beneath exhaustive consumers; it is not a shared mutable gate or a general incremental joined-result relation.

## Nonidentity flow composition

The existing identity-view shortcut remains. Larger nonidentity compositions can now retain resource and context unions across calls with the same immutable parent flow. Parent identity uses a weak pointer, preventing address-reuse errors while allowing the parent to be released. The source sets are exact keys. Output destinations, frame mapping and consumer-specific proof support are constructed normally.

Small compositions take the original local-cache path. The retained store is bounded, prunes expired parents and disables retention after record-pressure eviction. The focused repeated large-union fixture improves approximately 2× with cold controls approximately flat. This is reusable union construction, not a cache that merges whole contextual proofs or speculative rewrite templates.

## CPU admission

The executor still returns results in input order. The runtime still advances each selected task once and publishes results in its original order with unchanged work accounting. Default execution uses one worker.

Configured parallel execution now requires at least two substantial normalization tasks and at least 8,192 estimated world visits in a batch. Tiny search steps do not pay a worker handoff merely because multiple workers were configured. The initial estimate deliberately covers the measured normalization workload; it is not an automatic hardware optimizer.

The native sweep runs three successive normalization steps for 32 independent states. Four workers improve the measured 256- and 4,096-world batches about 2.8–3.5×. Small batches select serial execution. Construction and result validation occur outside these batch timings; this is not an end-to-end program speedup claim.

## Historical differential verification

`bazel test -c opt //benchmark:differential.test` builds both the current kernel and public revision `8c68ad69196f77c2601d294ac5c23b13505f2175` in one hermetic test. The archive is checksum-pinned and a development dependency; production targets do not depend on it. The patch only adds a separately named reference library. The historical frontend remains part of the reference build.

Generated cases cover nested rule production and rule matching through depth 32, capture, recursion, multiplicity, competing histories, 513-input exhaustive queries, nonidentity compositions, suspension and resumption. Full serialized direct reports and exhaustive snapshots are compared at multiple work budgets, excluding only exhaustive `record` and `peak`, whose cache footprint is intentionally different. Work, queues, states, token identities, captures, flows, evidence, support and outcomes remain compared. Cache-pressure cases force eviction before resuming.

This historical oracle complements the independent matching, reachability and support oracles already in kernel tests. It establishes checked equivalence to a named revision, not a proof that the historical interpreter or every possible program is correct.

## Residual pruning and the remaining research boundary

General residual assignment pruning has not been enabled. A Hall-style rejection can prove that no complete binding extends a prefix, but the old evaluator may still owe an observable sequence of pending steps below that prefix. For example, domains `A={0,1}`, `B={0,2}`, `C={0,2}` admit a complete assignment, while choosing `A=0` makes the suffix impossible. Removing that suffix changes bounded progress unless its exact waiting length and retained continuation can also be reconstructed. Token multiplicities and symmetry make that length depend on the concrete traversal, not merely the cardinality failure.

The implementation already has global assignment feasibility, capacity rejection before preparation, exact cached negative transcripts and waiting-span batching. The new gate cache further shares exact residual compatibility decisions without deleting steps. A stronger first-visit pruning algorithm still needs a cheap exact progress certificate; adding an unproven shortcut would contradict the preserved budget contract.

Other research boundaries remain explicit: arbitrary row-level incremental joins, complete persistent trace DAGs, parameterized contextual rewrite templates, and causal-history reduction. Binding payload links do not imply that all record headers are persistent. A union cache does not prove arbitrary contextual construction reusable. Inspectable competing histories cannot be discarded based only on equal terminal values. These are research tracks with additional equivalence requirements, not disabled language features or reasons to begin GPU work.

## Integrated acceptance

The [raw audit](cpu.json) preserves three alternating rounds against `8c68ad69196f77c2601d294ac5c23b13505f2175`, source hashes, all samples, power/sleep checks, focused fixtures and rejected code-generation experiments. The complete matrix is measured at `bf48e5f`; the subsequent constructor-only API tightening is recorded with its own three-round targeted audit, which also passes the stated performance gates. Gate sharing can only be attached during construction, so partially running gates cannot acquire misaligned cache handles.

| Workload | Before | After | Interpretation |
| --- | ---: | ---: | --- |
| Six factors of two | 66.47 ms | 65.33 ms | Essentially unchanged |
| Ten-digit decimal addition | 202.02 ms | 204.85 ms | Essentially unchanged |
| Five-digit decimal multiplication | 222.13 ms | 223.75 ms | Essentially unchanged |
| 4,096-frame lexical maintenance | — | — | 8.9× faster |
| Immutable trace snapshots | — | — | 3.3–55.9× faster |
| Parent binding composition | — | — | 1.1–10.8× faster |
| Repeated wide gate decisions | — | — | 4.0–10.9× faster |
| Repeated provenance unions | — | — | 2.0–2.1× faster |
| Large normalization batches, four workers | — | — | 2.8–3.5× faster |

Focused gate, union and CPU-admission ratios compare enabled and disabled paths in the same native build. Lexical and trace-storage comparisons use the named historical baselines. No cumulative multiplier is inferred from these figures.

The final full matrix has no pooled regression beyond its predeclared 5% protected-workload or 10% submillisecond tolerances. Larger individual-round outliers remain in the raw data. The initial integrated candidate regressed cold joins by approximately 13%; making the direct cursor boundary inlineable restored all six join controls to baseline or better. Outlining plan selection and inlining factor cursor wrappers did not resolve that regression and were reverted. The intermediate trace-payload negative-preparation regression also no longer reproduces beyond tolerance in the integrated build.

Validation passes all 107 optimized Bazel test targets, 201 debug kernel tests, formatting/lint checks, native/WebAssembly conformance and 619 generated historical budgeted observations. The constructor refinement passes the full suite again. Linux Buildkite verification also passes on the implementation revision; final publication is subject to the protected branch checks on the submitted head. These checks cover the shipped syntax and current dynamic executable occurrences. They do not turn the separate structural rule-generation proposal into an implemented language feature, or establish a universal absence of kernel bugs.
