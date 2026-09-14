# Runtime optimization investigation

The expression evaluator's main counted cost is matching, not arithmetic or canonicalization. The complete native example uses 10,017 rule applications but 3,411,783 internal work tasks at baseline commit `e03f978`. Temporary task instrumentation attributes 98.2% of those tasks to matching. This strongly supports investigating candidate selection and reuse before replacing canonicalization or increasing worker counts. It does not establish where 98.2% of CPU time goes: task durations differ, and substantial setup and reporting work occurs outside the counter.

Two small, general improvements are implemented: reject a particle match when any required term has no candidate, and retrieve direct-path event metadata without building an entire diagnostic snapshot. Neither change recognizes arithmetic, rewrites Photonic programs, merges labels, equates resource identities, or bypasses evidence projection. The larger indexing and incremental designs below remain proposals with explicit correctness obligations.

## Measurement and interpretation

A work step is an agenda task, not a processor instruction or an arithmetic operation. Search tasks advance either particle enumeration or the gate joining partial matches. Normalization tasks advance canonical ordering. Other tasks inspect views, deliver matches, attempt applications, or compose views. Batch execution counts every task individually. Running tasks on more workers therefore does not inherently reduce work.

The baseline task breakdown was measured with temporary counters at the existing task dispatch points. Instrumentation was removed before latency measurement; it is not a new runtime interface.

| Task | Baseline count |
| --- | ---: |
| Search | 3,348,683 |
| Normalize | 30,336 |
| Inspect | 10,017 |
| Deliver | 12,018 |
| Apply | 10,729 |
| Compose | 0 |
| Total | 3,411,783 |

These numbers describe one direct execution of `1212 * 10 / 2 + 11 - 1`. The intermediate decimal values remain 150, 75, 79, and 78. They do not describe exhaustive exploration of every expression execution. Zero Compose tasks in this path is not grounds for removing composition from the general runtime: source inference uses it.

The expression performance test also serializes and decodes the complete trace. Its elapsed time includes a different reporting workload from the CLI's compact text output. Timing these interchangeably would obscure the effect of runtime changes. The work counter likewise excludes final report construction, so reducing reporting overhead can improve time without changing work.

## Measured results

The retained timing run uses the same assembled input for all variants, one warm-up per variant, five measured samples, and rotating variant order. No other builds, tests, or benchmark workloads were run alongside these samples. This is a small development-host comparison, not a cross-platform performance guarantee. [Raw samples, platform, input hash, budgets, and reference measurements](investigation.json).

| Variant | Events | Work | Median compact CLI time |
| --- | ---: | ---: | ---: |
| Baseline `e03f978` | 10,017 | 3,411,783 | 3.334 s |
| Direct metadata projection only | 10,017 | 3,411,783 | 3.007 s |
| Projection and candidate pruning | 10,017 | 2,875,589 | 3.024 s |

Together, the changes reduce counted work by 15.7% and median compact CLI time by 9.3%. Projection alone accounts for the observed latency benefit. Pruning does not demonstrate an additional latency gain here: its median is about 0.6% above projection alone. It removes unsuccessful enumeration tasks, but candidate construction and other costs remain. This is a concrete reason to judge work and elapsed time separately rather than optimize the counter in isolation.

The existing 19 small reference benchmarks retain identical state and event counts between baseline and current runs. Their median per-case current/baseline latency ratio is 0.987; this sequential small-input comparison is descriptive and does not establish a broad speedup. Functional validation also compares reference state sets, exercises source inference and captures, and checks reports across worker counts and pause/resume schedules.

All 73 selected Bazel test targets pass, including the 101 Rust tests in `//system:test`, native expression and arithmetic cases, performance budgets, and the browser demo. These tests and the local rejection argument support the changes; they are not a universal machine-checked proof of the whole runtime.

## Implemented changes and correctness

### Reject impossible particle matches

`search::Particle::new` already computes the concrete token candidates for every required term. Previously, enumeration could descend through candidate prefixes even when a later required term had an empty candidate set. The change marks this particle search complete immediately when any set is empty.

The correctness argument is local: a complete assignment must select a token for every term. If one term has no candidate, no complete assignment exists. Removing all partial assignments from that search therefore removes no match. An empty pattern remains valid because it has no empty candidate set. Repeated terms still pass through the existing distinct-occurrence and symmetry checks. Atom matching still ignores capture, while rule values retain the existing exact capture requirement.

This avoids actual enumeration and queue traffic rather than redefining a work step. It does not yet reject insufficient multiplicity when every individual set is nonempty, or avoid scanning unrelated worlds. Those are separate opportunities.

The added independent subset oracle checks 5,440 combinations of three-token particles and patterns of length zero through three, including repetition, missing terms, and two captures of the same rule value. It checks complete output sets and rejects duplicate matches. Existing tests cover larger resumable enumeration, world joins, frame behavior, source inference, and deterministic parallel scheduling.

Fewer unsuccessful tasks can change when another search becomes ready. Consequently, complete semantic results and deterministic behavior across worker counts are the contracts to preserve; arbitrary intermediate reports at the same numerical work budget need not remain identical. The tested expression happens to retain its 10,017-event path. A bounded search remains Unknown when unfinished.

### Read event metadata directly

The direct-path runner previously called `Runtime::snapshot()` after every selected event and discarded everything except the first event's rule and binding metadata. Snapshot construction calculates support status and renders states, views, and events. That is useful for a report, but unnecessary for selecting the same stored transition.

A borrowed `Transition` now exposes the target, rule name, and immutable binding. The path report copies the same footprint, exact-selection, and read-support sets as the previous snapshot projection. Snapshot generation remains available and unchanged for callers requesting a complete report. There is no new scheduling rule or alternative execution mode.

The metadata test compares this borrowed projection against full snapshots for ordinary rules, joint inputs, generated code, and nested scopes. Existing path tests check pause/resume behavior and the distinction between direct execution and exhaustive inference. This change alone is expected to preserve event ordering and the work counter exactly; it removes allocations and reporting work outside that counter.

## Research relevant to the next changes

### Incremental pattern matching

Forgy's Rete algorithm maintains pattern-match information as working memory changes instead of repeatedly matching every object against every pattern. Its important contribution here is the architecture of retained partial matches and delta updates, not a prescription to adopt an entire production-system implementation.[^rete]

Photonic already shares matching caches across relative views for a target/frame/pattern inside one runtime. The direct-path runner, however, seeds a new runtime after each selected event. Retaining all old caches blindly would be unsound: consumed resources disappear, captures and frames can change, and canonical numbering can rename surviving objects. A correct incremental design must transport surviving bindings through explicit mappings and invalidate every dependency touched by an event.

A useful first step is an immutable per-state index, shared by all searches of that state. Posting lists keyed by frame, value, and relevant capture can exclude worlds that cannot match a pattern. This has a smaller correctness surface than carrying caches across transitions. Multiplicity must remain explicit; a set of labels alone cannot establish a multiset match. Empty patterns and rule-value captures need their own exact handling.

### Relational matching and join planning

Relational E-Matching formulates matching as conjunctive query evaluation and applies database join algorithms. Its results support considering selective indexes and join plans when backtracking explores many failed combinations. The paper's worst-case guarantees concern e-matching under its relational model; they are not automatically bounds on Photonic execution.[^relational]

Photonic's gate currently extends prefixes in pattern order and disallows reuse of a world within a binding. A selective join plan could reduce retained prefixes, but must map results back to original operand positions. Equal patterns also have symmetry restrictions that eliminate duplicate assignments. A reordered plan must preserve these restrictions and every valid binding, including distinct resources carrying equal values.

Start with indexes that preserve the current successful enumeration order. More aggressive join planning should be evaluated after identifying whether failed world scans or intermediate gate combinations dominate the remaining matching work. The current profile groups those together and cannot yet choose between them quantitatively.

### Differential and semi-naive execution

Differential Dataflow provides a model for updating iterative computations from changes, including partially ordered versions. It is relevant to deriving an explicit delta model for state evolution and evidence, especially when ordinary single-sequence cache invalidation is insufficient.[^differential]

Egglog combines Datalog and equality saturation and proves equivalence between its naive and semi-naive evaluation. Its delta-rule approach is a useful example of specifying the result first and proving that incremental maintenance computes it.[^egglog]

Neither system's correctness theorem transfers directly to Photonic. Photonic consumes resources, preserves shared introductions, tracks callable-code support, and projects inferred matches back to a source. A usable incremental design must cover all of those relations, including deletions and identity transport. Merely remembering that a textual pattern previously matched would lose necessary dependencies.

### Canonicalization and contextual reasoning

The egg paper's rebuilding technique amortizes restoration of e-graph invariants. It motivates asking which invariants can be maintained incrementally and where they must hold, but deferring Photonic canonicalization indiscriminately could alter state interning, cycles, resource mappings, and evidence.[^egg]

Practical Graph Isomorphism II is directly relevant to future exact canonical-labeling work. Photonic already has refinement and a sufficient swap certificate; any stronger pruning must preserve the chosen canonical state and the maps consumed by provenance. Equivalent-looking label multisets are insufficient.[^canonical]

Recent work on relational contextual equality saturation investigates sharing reasoning across contexts through layered equivalence relations. It illustrates the cost of copying contextual state and the importance of preserving context distinctions. The 2025 paper describes ongoing work, not a ready-made correctness result for captured Photonic environments.[^context]

These sources provide established and recent techniques, rather than a claim that a single current system is universally the fastest or semantically interchangeable with Photonic.

## Priority and acceptance criteria

| Candidate | Expected benefit | Required evidence before adoption |
| --- | --- | --- |
| Shared per-state candidate index | Avoid repeated scans of unrelated worlds and tokens | Exhaustive match equivalence, captures, multiplicity, empty patterns, memory accounting, latency |
| Multiplicity feasibility pruning | Avoid impossible repeated-term assignments | Independent multiset oracle and unchanged successful bindings |
| Selective gate joins | Reduce large partial-binding products | Complete binding-set equivalence, original operand roles, symmetry, pause/resume |
| Transition-aware cache reuse | Avoid rebuilding unaffected matches after each event | Explicit resource/frame mapping, dependency invalidation, fresh-code support, source inference |
| Incremental canonicalization | Reuse graph structure across local changes | Exact canonical state and provenance maps, shared/private resource adversaries |
| Worker batching | Reduce elapsed time for substantial independent jobs | Worker-count determinism, memory limits, useful batch sizes, measured overhead |

A sub-million target requires eliminating more than 70.7% of baseline work. The profile shows sufficient matching work to make that a plausible engineering direction, but it does not prove a particular implementation will reach it. The implemented pruning reduces work to 2,875,589, leaving approximately 65.2% of that count to remove to reach one million. No theoretical lower bound above the target has been established.

The direct path's one-task advancement and runtime reseeding currently prevent it from benefiting from the existing worker pool. Increasing the worker count without changing this execution structure cannot establish a speedup. Any future batching design must specify the event-selection contract and ensure that speculative work does not silently change resource limits or completeness claims.

The safest next substantial experiment is the shared per-state index. It addresses the measured matching bottleneck while keeping source inference, application, and canonicalization intact. Cross-event incremental reuse should follow only after identity transport and invalidation have a precise contract. No arithmetic recognition, native evaluator compiler, label collapse, or weaker semantics is needed for either direction.

## Reproduction

Build and test with the pinned Bazel toolchain:

```sh
bazel test -c opt //system:test //mathematics/ternary:all //tool:check //tool:browser --test_output=errors
bazel run -c opt //system:benchmark
bazel run -c opt //mathematics/ternary:infix -- obsidian \
  --target "$PWD/mathematics/ternary/result.particle" --path \
  --steps 100000000 --states 65536 --cells 16384 --frames 2048 \
  --coherences 1024 --records 100000000
```

For elapsed-time comparisons, build each revision first and invoke its compiled command directly with the same assembled program and budgets. Exclude Bazel build time, use a warm-up, rotate revision order, and retain individual samples. Compact CLI measurements include process startup, input decoding, execution, final path-report construction, and compact output. They exclude full JSON trace encoding. Measure that reporting workload separately when it matters.

## Sources

[^rete]: Charles L. Forgy. [Rete: A Fast Algorithm for the Many Pattern/Many Object Pattern Match Problem](https://www.csl.sri.com/users/mwfong/public_html/Technical/RETE%20Match%20Algorithm%20-%20Forgy%20OCR.pdf). Artificial Intelligence 19, 1982, pp. 17–37. Sections 2–3 describe retained matches and updates.
[^relational]: Yihong Zhang, Yisu Remy Wang, Max Willsey, and Zachary Tatlock. [Relational E-Matching](https://arxiv.org/pdf/2108.02290). POPL 2022. Relational formulation, join planning, and data-complexity results.
[^differential]: Frank McSherry, Derek G. Murray, Rebecca Isaacs, and Michael Isard. [Differential Dataflow](https://www.cidrdb.org/cidr2013/Papers/CIDR13_Paper111.pdf). CIDR 2013. Sections 3–5 describe difference traces, iteration, and scheduling.
[^egglog]: Yihong Zhang et al. [Better Together: Unifying Datalog and Equality Saturation](https://arxiv.org/pdf/2304.04332). PLDI 2023. Section 4.3 and Appendix B cover semi-naive evaluation and its correctness.
[^egg]: Max Willsey et al. [egg: Fast and Extensible Equality Saturation](https://arxiv.org/abs/2004.03082). POPL 2021. Rebuilding and invariant restoration for equality saturation.
[^canonical]: Brendan D. McKay and Adolfo Piperno. [Practical Graph Isomorphism, II](https://arxiv.org/abs/1301.1493). Journal of Symbolic Computation 60, 2014, pp. 94–112. Exact graph isomorphism and canonical labeling.
[^context]: Tyler Hou, Shadaj Laddad, and Joseph M. Hellerstein. [Towards Relational Contextual Equality Saturation](https://arxiv.org/pdf/2507.11897). 2025 preprint. Context-indexed equivalence relations and ongoing relational implementation work.
