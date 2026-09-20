# CPU research before GPU execution

Research checked on 2026-09-20. The [interior matching audit](interior.md) describes the implementation delivered with this review; the [roadmap](roadmap.md) distinguishes shipped work from research. The recommendations below are an assessment of applicability to Photonic, not performance claims imported from other systems.

The strongest remaining CPU opportunities are subscription and immutable preparation reuse, a bounded graph of maintained matching fragments, and execution over compact batches. Keep the Rust kernel as the semantic authority. A graph database is not a replacement for occurrence identity, synchronized input selection, captures, resumable progress, or proof provenance.

## Current expression bottleneck

A follow-up native instrumented run of `2*2*2*2*2*2` on the delivered implementation takes about 70 ms. Dispatch accounts for about 38 ms, index maintenance 13 ms, rewrite 7 ms, and the measured matching phase 2 ms. Within dispatch, subscription work accounts for about 26 ms; query preparation accounts for about 11 ms within subscription. These scopes are nested and must not be added as independent costs. The [audit artifact](interior.json) retains all seven diagnostic samples and their source revision; ordinary, uninstrumented arithmetic timings remain in the paired audit.

This changes the practical priority: first investigate subscription reconciliation and reuse of immutable query/particle preparation on real expressions, then expand maintained fragment sharing where it saves measured work. The synthetic join improvements have moved ordinary arithmetic by only about 1–2%. A hypothetical infinitely fast implementation of the measured matching phase would save only about 2 ms in this diagnostic, before GPU overhead. This is an inference from one direct-execution workload, not a bound for every program or the exhaustive runtime. Re-profile both execution modes before choosing a GPU kernel.

The subsequent [preparation audit](preparation.md) implements the immutable particle portion of that priority and removes repeated occurrence/ordinal conversions. A focused repeated-preparation workload improves roughly 20–40×, while the paired expression workload improves about 2%. Subscription reconciliation remains substantial. The subsequent [multilevel audit](hierarchy.md) projects irrelevant ancestor context out of cache keys and retains a bounded forest of short fragments. Deep alternating-update workloads improve about 6–12× against the preparation revision; arithmetic is approximately unchanged. Exhaustive fragment sharing remains open.

## What the implementation now represents

Photonic contains several hypergraphs. Worlds group resource occurrences; firings synchronize several inputs; proof clauses require all premises. Alternative proofs and competing histories add different dependency relationships. A matching plan is a reusable computation graph over this data, not the state itself. The [architecture report](architecture.md) separates these identities and owners.

The current cache retains either one partition or several observed interior depths. Keys contain the anchor occurrence and only ancestor occurrences that could affect descendant world distinctness or symmetry. Transcripts store the local suffix; replay restores each consumer's exact inherited binding. Reverse dependencies retract removed occurrences, while explicit searched-domain intervals invalidate regions that may gain or lose answers. Admission keeps long fragments on the simpler single-depth path. The result is a bounded per-join forest whose parent transcripts copy child records, rather than a persistent child-link DAG or cross-plan partition graph.

Whole-prefix consumers can also share an immutable snapshot before enumeration finishes. Each consumer retains its own delivery position. A follower can take a longer snapshot when available; otherwise it restores its private cursor and continues. The producer is never advanced speculatively by a follower. This preserves the current polling contract, while leaving cooperative production and efficient continuation checkpoints open.

## Recommended order

| Priority | Work | Why it can matter | Gate before production |
| --- | --- | --- | --- |
| 1 | Measure actual cursor work, fragment survival, frontier overruns, allocations, and phase time | Distinguish repeated matching from output/provenance cost | Disabled instrumentation has no hot-path cost; separate direct and exhaustive execution |
| 2 | Reduce subscription reconciliation; share immutable preparation and compact dependency keys | Reuse below plans without duplicating mutable enumeration | Exact tokens/captures, site reuse, private reset and eviction; cold and low-reuse controls |
| 3 | Maintain a multilevel fragment graph with explicit searched-domain dependencies | Avoid dropping an entire partition when another interior input changes | Coherent multi-input deltas, negative fragments, independent delivery, bounded discovery and storage |
| 4 | Add adaptive residual filtering and physical join choices | Avoid large unsuccessful intermediate products under skew | Preserve occurrence multiplicity, symmetry, exact bindings and logical suspension boundaries |
| 5 | Extend shared fragments into exhaustive matching and contextual construction | Reuse across proof consumers rather than only direct dispatch | Source inference, executable reads, capture mapping and competing evidence remain complete |
| 6 | Compact storage and bulk execution, then CPU parallelism | Reduce allocation, dispatch and memory traffic; expose useful batches | Exact budgets, cancellation, observation and synchronization on native and WASM |
| 7 | Measure GPU crossover on a proven bulk primitive | Accelerate enough uniform independent work to amortize submission and transfer | CPU oracle, bounded output buffers, coherent updates and complete fallback |

The order is conditional on measurements. In particular, a faster candidate preparation layer should precede a general optimizer if preparation dominates real arithmetic. A large theoretical improvement to a phase taking 2% of execution cannot yield a large end-to-end speedup.

## Maintained fragments rather than larger transcript caches

For a plan with inputs `A, B, C, D`, retaining each `B` region under its exact `A` binding allows one changed `B` occurrence to leave the other regions intact. A change to `C` invalidates any layer whose searched interval contains `C`, while compatible deeper layers can survive. A future representation could share maintained relations across plans and link parent records to children without copying, with explicit edges recording both selected occurrences and searched predicates.

An emitted row depends on positive selections. An exhausted region also depends on everything that could have produced an answer. Recording only the tokens found in successful bindings is therefore unsound for negative reuse: inserting a newly matching world must reopen the relevant region. Context changes need the same treatment. A fragment interface must separate selected identities, searched-domain versions, residual constraints and consumer projection.

[DBSP](https://arxiv.org/abs/2203.16684) provides a compositional basis for incrementalizing relational and recursive computations. [Heavy-light query maintenance](https://arxiv.org/html/2605.08397v1), a May 2026 preprint, combines delta queries, view trees and degree partitioning to obtain improved update bounds for general joins. These support moving from whole-query invalidation toward maintained intermediate relations. Their assumptions do not establish Photonic equivalence: an update can change several domains, read eligibility and captures simultaneously, and exact occurrence/proof identity must survive.

Start with a bounded set of exact subplans already exposed by compiled rules. Retain their boundary mappings and generation-sensitive dependencies. Use one coherent update transaction and a lazy consumer cursor over maintained fragments. Add skew-aware heavy/light decomposition only after simple view maintenance demonstrates a bottleneck; its rebalancing and discovery costs are real, and its bounds concern query families and data size rather than arbitrary reflective programs.

[Shared Arrangements](https://www.vldb.org/pvldb/vol13/p1793-mcsherry.pdf) is the closest systems precedent for sharing maintained indexed state among independent queries. The first practical application here is immutable preparation: several consumers can share candidate/token data while keeping their own combination cursor. Extending ownership at this boundary is less invasive than sharing a mutable search producer.

## Adaptive joins and early rejection

[Free Join](https://arxiv.org/abs/2301.10841) unifies traditional and worst-case-optimal joins instead of assuming one always wins. [A Unified Architecture for Efficient Binary and Worst-Case Optimal Join Processing](https://arxiv.org/html/2505.19918v1) extends the physical choice across hash and sorted representations. [Adaptive Factorization Using Linear-Chained Hash Tables](https://vldb.org/cidrdb/2025/adaptive-factorization-using-linear-chained-hash-tables.html) explores runtime statistics and comparatively simple heuristics for deciding when factorization pays.

For Photonic, this suggests a small physical planner driven by candidate size, overlap, reuse, surviving-fragment ratio and actual rejection cost. Retain the current direct cursor for small work. Consider sorted intersections, factorized extension, or a trie only for constraints that benefit. Do not transplant a database's relational join assumptions into distinct-resource assignment. Sparse, dense and skewed domains need separate controls, and a changed traversal must still reproduce required suspension and observation behavior.

The August 2026 preprint [Uplifting the Superpowers of Worst-Case-Optimal Join Algorithms](https://arxiv.org/html/2608.03840v2) integrates filters into traversal over compact graph indexes. Its useful lesson is to test selective constraints while narrowing candidates, rather than build a large intermediate result and filter afterward. Photonic already uses presence filters and exact particle matching; the remaining opportunity is residual capacity/distinctness filtering under partial bindings. Graph-query benchmark speedups do not predict Photonic speedups. The paper's compact property-graph index also does not supply Photonic's dynamic capture or proof machinery.

## Abstraction sharing and proof construction

[FlowLog](https://arxiv.org/html/2511.00865v3), presented in VLDB 2026, separates recursive control from relational plans and shares normalized subplans. This is a useful architectural model: intern immutable operators with exact parameter interfaces, let contexts specialize them, and keep the recursion/proof engine outside the matching operator. Hashes should discover possible reuse; exact equality must authorize it.

[Subsumptive tabling](https://www.swi-prolog.org/pldoc/man?section=tabling-subsumptive) explains how a general query can serve specific queries. [Incremental tabling](https://www.swi-prolog.org/pldoc/man?section=tabling-incremental) adds dependency invalidation and lazy reevaluation. These are close analogues for the proposed abstraction-to-instance hierarchy. A general result is reusable only when its projection covers the consumer's exact resource and capture requirements. Lexical nesting by itself is insufficient.

Exhaustive matching still needs its own consumer-specific source inference and flow projection. A reusable proof construction template must parameterize the source, selected resources, executable reads, captured environment and fresh identities. Start with exact nonidentity flow composition or a measured repeated construction pattern. Do not equate equal output labels with equal proofs.

The September 12, 2026 revision of [The Role of Semirings in Incremental View Maintenance](https://arxiv.org/abs/2606.07795v2) makes a particularly relevant theoretical point: maintenance complexity depends on the annotation algebra. Its results concern insert-only annotated conjunctive queries under stated assumptions. Photonic cannot adopt Boolean existence in place of resource multiplicity or competing provenance simply because a Datalog implementation finds that specialization profitable.

## Progress, allocation and parallel work

The current transcript caches compress repeated `Pending` values in storage but still deliver each logical step. Consequently, they reduce matching cost per step while retaining a lower bound proportional to delivered progress. A batch interface could represent a run of equivalent pending work compactly and let an executor consume part of it, but it must preserve all observable budget boundaries, agenda rotation, state changes, cancellation and resumptions. Skipping a known number of internal steps is safe only with a certificate that those steps cannot change an intervening observation or another consumer's behavior.

This is an implementation hypothesis to investigate, not permission to redefine `run(n)`. Begin with trace replay, compare every prefix budget against the scalar executor, and measure executor overhead independently. Returning the same eventual answer is insufficient. Generic fusion must also preserve the exact accounting and projection boundaries of the fused operations.

Measure allocator bytes as well as logical records. Compact immutable token arrays, shared prepared patterns, pooled dependency storage and fewer temporary binding copies can improve cache locality without changing the state model. Use generations where pooled numeric slots can be reused. Native architecture-specific acceleration requires a portable WASM path and should apply to generic intersections or scans, not recognized arithmetic programs.

CPU parallelism should first operate on independent immutable preparation or indexed filtering within a coherent snapshot. Publishing a match and committing a synchronized firing remain separate operations. GPU kernels become plausible when these operations expose sufficiently large uniform batches, with explicit output capacity, versioned inputs, cancellation and overflow fallback. Device throughput alone is not the metric; compare complete execution, including dispatch, data movement and result integration.

## Larger state-space reductions

[Unfolding-based Partial Order Reduction](https://arxiv.org/abs/1507.00980) combines causal representations and independence to avoid redundant concurrent executions. This could yield a much larger reduction than accelerating individual matches when independent firing orders dominate exploration.

It is also the highest semantic risk. Photonic's executable reads, captures, competing resource uses and inspectable histories all constrain independence. A future causal representation must reconstruct every required proof/report observation. Reducing schedules solely because their final labels agree is not acceptable. Treat this as a separate equivalence project with an explicit observation model, not a prerequisite that must be forced into the kernel before GPU work.

No implementation defeats genuinely exponential required output or evidence. The practical aim is to remove repeated discovery and reconstruction, keep shared structure compact, and spend work in proportion to changed dependencies and demanded observations. The maintained-fragment forest is now measured with explicit validity boundaries. Residual rejection, exhaustive integration and exact bulk execution remain the next conditional stages; no claim that every possible optimization has been completed follows from these benchmarks.
