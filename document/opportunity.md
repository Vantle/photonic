# Architectural opportunities

Assessment on 2026-09-21 against `bdc4de1`. This supersedes allocation tuning as the immediate priority in the [roadmap](roadmap.md). The objective remains execution of the same rules and programs with unchanged semantics and public contracts. Scheduling may change. Rule construction, output replacement and automatic program rewriting remain outside scope.

The first representation slice is now delivered in [shared matching structure](plan.md). [Incremental report materialization](view.md) is also delivered across direct-path, exhaustive and prism JSON output. The [activation audit](activation.md) adds lifecycle attribution and deferred enumeration for initially empty candidate domains. [Compiled scope membership](scope.md) now uses flat declaration groups and compact masks, improving scope scaling while leaving arithmetic approximately flat. General subscription membership maintenance remains open. The baseline findings and estimates below record the evidence that motivated these changes.

The subsequent [subscription retention experiment](subscription.md) rejects eager maintenance of parked whole searches. Despite many potential reuse hits, its bounded prototype is about 83–93% slower on four complete expression workloads and has been removed. The remaining membership opportunity requires separating lightweight membership from demanded candidate state; the admission counts do not justify a general query-retention cache.

The [empty search representation experiment](dormancy.md) subsequently rejects both boxed and inline compact wrappers. The boxed six-factor case is about 7.1% slower; inline timing differences are below 1.5%, with negligible allocation savings. Differential activation and replay tests remain. Identify actual construction or lookup cost before adding another membership representation.

## Assessment

The runtime has substantial algorithmic optimization already: interned input structure, incremental candidate domains, bounded maintained fragments, immutable preparation, exact transcript replay, persistent history, incremental support and cost-based CPU admission. These are working foundations, not a backlog to implement again. The [continuation](continuation.md), [sharing table](roadmap.md#current-sharing-and-remaining-gap) and later audits distinguish their limits.

The remaining architectural weakness is repeated materialization between layers. Dispatch repeatedly builds context-bound searches from shared inputs. Export builds a complete owned presentation of retained history before serialization. Parent traces share payloads but still reconstruct composed records. Some query consumers share preparation without sharing maintained interior results. Each boundary offers an opportunity to remove a category of repeated work.

This is not evidence of a globally optimal or finished runtime. The existing regression suite and measured improvements establish particular cases; they do not establish unlimited scaling or universal semantic correctness. Equally, a new cache or more modules would not automatically improve the architecture. Prefer fewer representations, explicit ownership and independently testable contracts.

## Priorities and estimates

Effort is a planning range for one experienced engineer, including validation. Numerical benefits below are experiment targets or conditional calculations, not forecasts. Do not add them together.

| Priority | Architectural change | Value and uncertainty | Initial effort |
| --- | --- | --- | --- |
| 1 | Separate reusable matching structure, bound context and live subscription state | Best measured compact-execution opportunity. Target at least 10% lower complete execution latency for substantial new reuse machinery; actual benefit is unknown | 1–2 weeks for one validated slice; 3–6 weeks if lifecycle maintenance is needed |
| 2 | Serialize history through a report view without retaining every rendered node | Clearest export-memory opportunity. The measured owned report is about 108 MB; aim to remove most of that temporary storage for streaming consumers | 1–2 weeks for direct-path export; 2–4 weeks including other report consumers |
| 3 | Share maintained interior relations across independent queries | Largest conditional scaling opportunity when many consumers repeat the same joins; no credible whole-program multiplier yet | 1–2 weeks to establish demand; 4–8 weeks for one integrated join family |
| 4 | Compose trace segments lazily with independent continuation cursors | Avoid work proportional to every child record at every parent extension; strongest on deep or repeatedly composed traces | 2–4 weeks for a bounded direct-path implementation |

Priority 1 serves execution, priority 2 serves reporting, and priorities 3–4 need their own representative workloads. They are not four mandatory projects. A simple ownership improvement may be worthwhile without a large speedup; a complicated reuse mechanism must earn its maintenance cost.

## 1. Reusable structure and subscription lifetime

The [registry audit](registry.md) measures approximately 41 ms in dispatch within a 72 ms instrumented expression execution. Subscription reconciliation accounts for about 27 ms inside dispatch, including roughly 10 ms of preparation and 15 ms of admission inclusive of preparation. Matching takes about 2 ms. The scopes overlap. Hypothetically halving subscription time would reduce total latency by about 19%, or give about 1.23× throughput for serial identical runs; this is an Amdahl illustration, not a prediction. Eliminating matching would save only about 3% on this workload.

A new count-only diagnostic of `2*2*2*2*2*2` finds:

| Observation during execution | Count |
| --- | ---: |
| Firings | 10,101 |
| Search admissions | 66,492 |
| Distinct frame/input/owner coordinate combinations admitted | 2,110 |
| Distinct input/owner coordinate combinations admitted | 486 |
| Distinct compiled inputs admitted | 341 |
| Admissions with captured patterns | 37,027 |

The warmup and subsequent execution agree on these counts. Diagnostic logging is deliberately excluded from latency evidence. The [artifact](opportunity.json) retains the patch, count distribution, observations and reproduction command. Repeated numeric coordinates do not prove equivalent live contexts: frames and occurrences may be replaced. The counts justify investigating repeated setup, not caching completed searches by these coordinates.

The current code exposes a concrete separation to improve:

- [Input](../language/plan.rs) already shares immutable fragments and symmetry groups. Its `pattern(owner)` nevertheless allocates a complete bound term matrix for a captured input.
- [Join](../language/joining.rs) passes both that matrix and a parameterized `Context` to its constructor.
- [Space](../language/joining/space.rs) holds both descriptions alongside mutable candidate members, particle cursors and subscriptions.
- [Reconciliation](../language/dispatch/subscription.rs) preserves entries with surviving keys, but removal and subsequent admission reconstruct searches. [Consumer discovery](../language/dispatch/consumer.rs) enumerates and sorts a requested consumer list for each selected frame reconciliation.

The first slice should make immutable input shape and owner parameters the authoritative matching description. Derive bound terms only at boundaries that actually require them. Keep candidate membership, enumeration, executable reads and consumer delivery private. Remove redundant production representations after differential validation. This is parameterized matching of the original rule, not rule specialization or reconstruction.

Then measure what remains. Distinguish genuinely new subscriptions, recurring coordinates with changed identity, unchanged consumer membership, candidate-domain reconstruction and particle preparation. A coherent delta can update affected memberships directly where the current path rediscovers a full request list. Existing trigger indexes, frame-local registries and reverse lexical dependencies already perform part of this job; extend their ownership boundary rather than adding a second dependency system.

Do not assume that removing captured-pattern allocation alone achieves the latency target. Eager capture preparation previously failed acceptance in the [reconciliation audit](reconciliation.md). Retaining dormant mutable searches is a different and much riskier project: it needs precise invalidation, bounded storage and contractual record accounting. Agenda restart costs only about 0.24 ms here, so a new persistent scheduler alone cannot deliver the intended benefit.

Acceptance must include capture-free and captured programs, cold and changing contexts, equal code with different evidence, coherent insertion/removal, slot reuse, new subscribers, eviction and every relevant pause boundary. Preserve the existing consumer and logical progress contract. If scheduling changes, test permitted order variation explicitly rather than silently weakening exact comparisons.

## 2. Report views and incremental serialization

[Search::report](../language/path.rs) retains canonical history and creates a `Vec<Node>` containing every rendered state, plus owned event copies. The [CLI](../command/main.rs) then serializes the completed report to a writer. Streaming bytes is already implemented; streaming report materialization is the missing boundary.

The [preservation audit](preservation.md) attributes roughly 130 MB to the engine, 108 MB to the owned report and 134 MB to an encoded buffer, with about 372 MB peak requested storage in the full-export benchmark. Avoiding the entire owned report would remove at most approximately 29% of that measured peak before replacement overhead. The CLI does not retain the encoded buffer, so its memory measurement must be separate. These are requested heap bytes, not process RSS.

Introduce a report view over a consistent search snapshot, with a serializer that visits states and events incrementally. A first slice can render and release one node at a time; a later slice can serialize canonical fields directly if rendering allocation remains significant. Keep one rendering implementation shared with owned snapshots, and retain an explicit owned snapshot operation for consumers that need the result to outlive the engine. These are different ownership requirements, not duplicate evaluator implementations. Serde's [compound serialization interface](https://serde.rs/impl-serialize.html) supports incremental sequence emission without first constructing a sequence container.

Preserve field order, exact serialized content, canonical mappings, event provenance, owned-report independence and report-then-resume behavior. Existing reporting initializes canonical caches, so a new view must account for the effect on subsequent budgeted normalization. Do not claim constant total memory while the engine still retains history and canonical forms. Writer failure also needs an explicit, tested boundary.

A strong initial target is at least 25% lower peak requested bytes for full export without an export-latency regression. Canonicalization is substantial, so removing rendered nodes alone does not promise a proportional time reduction. Measure export-to-writer and export-to-buffer separately and retain compact execution as a control.

## 3. Shared maintained relations

The existing implementation shares candidate filters, immutable preparation, direct-path fragments and exact exhaustive replay. It does not provide a general maintained interior relation shared across independent proof queries with different private projections.

The opportunity is to maintain one useful intermediate result under changes and let several consumers traverse it independently. This can reduce repeated join work as consumer count grows. [Shared Arrangements](https://www.vldb.org/pvldb/vol13/p1793-mcsherry.pdf) supplies a systems precedent for shared indexed state with separate consumers; [DBSP](https://arxiv.org/abs/2203.16684) supplies a compositional treatment of incremental computations. Neither establishes equivalence for Photonic's occurrences, captures, competing evidence or suspension contract, and neither paper's speedups predict ours.

Start with one exact subplan already exposed by compiled inputs and two real consumers. Define its input dependencies, capture parameters, occurrence generations, output mapping and consumer-specific evidence boundary. Apply coherent changes before publishing results. Record dependencies of exhausted regions as well as successful rows, so insertion can invalidate an old absence result. Bound discovery and retention, and compare against direct evaluation when sharing is absent or churn is high.

Require scaling evidence across increasing consumer count and mutation density, including cold installation, warm reads, retraction, late subscription and eviction. A large synthetic join gain would be valuable for that workload, but should not be described as a large improvement to today's expression benchmark, whose measured matching cost is already small.

## 4. Lazy trace composition

Persistent binding payloads and chunked transcripts have shipped. However, [Trace::extend](../language/joining/trace.rs) still visits child records and constructs parent records with inherited prefix links. A segment graph could retain a child transcript and its prefix projection once, while an independent consumer cursor expands the logical sequence on demand.

This targets repeated parent composition rather than already cheap transcript snapshots. Keep exact pending spans, binding multiplicity, completion boundaries, logical retained accounting and eviction restoration. Physical sharing must not silently alter budget behavior. Bounded depth or an iterative traversal must prevent the representation from introducing stack failure during replay or destruction.

Measure total construction plus delivery and release across trace depth and reuse, not only the cost of constructing a lazy wrapper. If consumers immediately demand every record once, deferred work may offer little benefit. The initial success criterion is materially less composition work on the measured long-trace family with no regression in complete consumption or compact execution.

## Novelty, maintenance and stopping

The most distinctive research direction remains a certificate that proves the exact logical waiting length of a rejected search region and can restore every interrupted continuation. That could avoid first-visit enumeration, beyond replaying an already cached transcript. The [certificate investigation](certificate.md) explains why simple rejection is insufficient. Research originality is unestablished; give a restricted prototype 1–2 weeks only when a workload spends substantial time in such regions.

Maintenance work belongs inside these architectural slices: one owner for each lifetime, explicit identity distinctions at state boundaries, accounting owned by the structure it measures, and a small independent oracle. Avoid a wholesale module or crate split without a concrete dependency boundary. Preserve Bazel-only hermetic builds and native/WebAssembly semantics, while accepting performance on the current development machine.

The immediate plan is the remaining subscription membership work in priority 1, guided by the activation and retention audits. Measure admission cost by input shape before choosing a compact inactive representation or broader shared domain maintenance. Priority 2 is delivered. Defer general maintained relations and lazy composition until representative workloads justify them. GPU execution, general parallel rewrite, causal pruning and a universal optimizer are not current prerequisites. There is no requirement to implement every idea to have a robust runtime.

Low-cost allocation improvements remain admissible when clear and repeatable. The final preliminary direct-candidate-collection probe improved expression latency by 2.1%, regressed addition by 2.8% and improved multiplication by 0.4%. It was removed without a full acceptance audit; its patch and paired samples are archived in the artifact. This assessment changes the plan, not production execution behavior.
