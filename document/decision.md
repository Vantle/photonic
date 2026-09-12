# Closing the runtime design

This records the accepted resolution of the remaining decisions in the [runtime plan](plan.html). The finite ground Rust runtime and JavaScript reference now implement these core decisions: generated activation, read/consume projection, whole-rule replacement, capture, and conditional availability. Native lowering, CLI execution, resumable searches, refined canonicalization, component-local support, and deterministic workers are implemented. Arbitrary structural operations are outside the accepted core; strict resource isolation remains a future boundary. Tests are not a proof of unrestricted correctness. The [configuration contract](semantics.md) continues to describe the tested finite evaluator.

## Rule availability and consumption

Use the same event operation for ordinary rules and rules produced by computation. A visible rule is a premise of its application. Invoking it reads its availability; explicitly matching it as an operand consumes its occurrence. No global rule installation or separate meta evaluator is needed.

Record two source projections: C, the footprint consumed by the operand binding, and R, the footprint read to establish rule availability. Preserve the concrete source outside C. In particular, preserve R minus C. When one occurrence participates in both, consumption wins; reading it does not restore it. Evidence and conditional dependencies remain attached to the event regardless of whether an occurrence survives.

Read support establishes an event at its historical source. Consuming that resource in a later successor does not retroactively defeat the earlier event. Defeating a logical assumption used by that event is a different operation and may change conditional support. Do not use present-day liveness as the truth condition of historical reads.

This distinction is bookkeeping about existing application roles, not a proposed language keyword or a special kind of type computation. Static rules currently hide their availability in the compiled scope table. Making the same premise explicit permits generated rules to use the ordinary event mechanism.

The projection must retain separate consumed-flow and read-support maps through composition. A single union of all causal ancestors cannot implement this contract: it would accidentally consume data merely because it justified a rule. A returned body's held consumption belongs to the consuming flow; lexical availability belongs to support. The body-transfer convention still preserves concrete non-exact witnesses as specified in the existing contract. A body created by an inferred generated rule retains its executable closure and read dependencies as frame metadata. It does not allocate a consumable R occurrence just to preserve the definition.

Write R for the whole value of the rule A → B. The following is explanatory notation, not source syntax.

| Program and starting state | Actual execution | Inferred source application |
| --- | --- | --- |
| Seed → R; start Seed.A | Seed.A → R.A → R.B | Seed.A → Seed.B; Seed supplies read support, A is consumed |
| Seed → R.A; start Seed | Seed → R.A → R.B | Seed → B; the same Seed supports the rule and supplies the consumed operand |
| Seed → R or Seed → A; start Seed | Alternative configurations R and A | No B: no joint witness supplies both rule and operand |
| Meta explicitly matches R and produces S; start R.X | R.X → S.X | Whole-value replacement preserves unrelated X and historical R.X |

The second row has no inferred surviving R, because R is evidence-only at Seed and Seed was consumed. The first row has no inferred surviving R either, but preserves the concrete Seed. These differences are intentional consequences of source inference. Do not manufacture code values to make inferred and sequential results look identical.

A reusable generated rule can consequently have a reusable concrete source justification. That may permit further computations on fresh operands. It must not duplicate an unchanged event: source, rule closure, read binding, consuming binding, continuation, and support identity determine application reuse. Test this property explicitly instead of imposing a special ban on repeated meta applications.

## Local activation and lexical identity

A rule value becomes eligible when exposed in an active coherence or lexical environment. Mentioning a rule inside another rule's pattern or an inactive body does not execute it. The rule and arguments must belong to one joint witness, under compatible scope and continuation conditions.

For a local rule occurrence, its coherence is the execution site. An ordinary one-coherence application binds there. A multi-coherence application includes that site among its participating input coherences; it cannot borrow a rule from an unrelated coherence. A zero-input rule still has an execution site. A lexically inherited rule is supplied by the active environment and carries that environment's support.

Track coherence participation independently from the read and consuming resource sets. Context required only to justify a rule remains where it is; reading cannot implicitly decohere it or broadcast its contents. Only coherences bound by the rule’s input positions participate in remainder transfer, including an explicitly bound empty coherence.

Returning a value and exposing a nested body are already distinct output behaviors in the reference. Keep those behaviors explicit in the intermediate representation. A rule value moved between scopes retains its defining environment. Its return destination is supplied by invocation, not confused with its captured environment.

Rule code is immutable. Updating an available definition creates a successor environment or consumes and replaces a local value; it does not mutate environments already captured by historical closures. This prevents a later local rewrite from silently changing an earlier function value. New closures capture the successor environment. If live rebinding is wanted, a program must represent and evolve that binding explicitly.

Canonical identity includes the rule structure and its captured environment graph. Source spans, display names, allocation addresses, and temporary frame numbers are not semantic identity. Normalize anonymous graph identities and multiset order while preserving multiplicity and sharing. Cyclic environments require finite graph canonicalization, not recursive tree expansion. Root/global atom identity remains exact; no arbitrary equivalence of rule behavior is inferred.

## Abstractions over whole values

Keep exact matching as the primitive. A rule containing another rule occupies one value position. Interning can make exact comparison cheap without decomposing the value into live operands.

Continue to express abstractions as ordinary derivations. A program can derive a description of a whole rule and match that description at its concrete source, just as True derives Boolean. The concrete remainder/body convention provides the witness. No mandatory structural wildcard or second type matcher is needed for this core.

This does not prove that every desired structural transformation can already be expressed. A transformation that extracts an arbitrary unknown input or body needs an explicit program-expressible representation and operations for those components. Before adding native capture variables, construct representative map, composition, and recursive abstraction examples in the structural value model. If those require new structure access, define it as ordinary value operations with exact contracts, not an implicit evaluator escape hatch.

## Absence and changing rules

Use conditional support as the general interpretation. An absence premise names a source, scope, pattern, and executable environment. New evidence can change its support; absence never means a timeout or a momentarily empty gate.

A closure certificate is valid only for the dependencies under which it was established. Those dependencies include available and potentially producible rules. Producing a rule that can derive Q invalidates an earlier certificate that assumed no route to Q, even before a Q occurrence appears. Mark the affected obligation open and propagate changed support. Independent justifications remain intact, and historical results remain recorded.

Track certificates by dependency components, with conservative invalidation when dependencies cannot yet be bounded precisely. Over-invalidation costs work; failing to invalidate changes meaning. A component can close when its relevant rule/value generators and agenda obligations are complete, or when a sound invariant excludes the query. Unbounded generation may leave it open forever while unrelated work continues.

The fixed compiled-rule label closure is disabled whenever rule values occur in the reference program. Dynamic queries conservatively remain open until supported evidence or finite completion resolves them. Early component-local certificates remain an optimization to implement. Keep negative cycles conditional rather than selecting a stable interpretation by evaluation order. SWI-Prolog's [well-founded semantics](https://www.swi-prolog.org/pldoc/man?section=WFS) and [incremental tabling](https://www.swi-prolog.org/pldoc/man?section=tabling-incremental) provide useful precedents; Molten uses its own resource and source-projection contract.

## Surface syntax

The executable [native grammar](syntax.md) lowers into explicit atoms, whole rule values, particles, coherences, and bodies. Dots combine particle values; commas separate coherences; semicolons terminate initial configurations and definitions. A rule has bracketed inputs and an explicit `->` output.

`@([A] -> B)` returns or matches a whole rule value. `{ ... }` enters a body containing an optional initial particle and local definitions. `()` denotes one empty particle; `[]` denotes zero input or output coherences. The distinctions are represented explicitly in `source::Program` and covered by lowering and CLI tests.

The abbreviation `[[A,B]] ([B])` remains structurally parseable but is not executable native syntax. Its omitted outputs and value/body ambiguity do not acquire meaning from the structural parser. Use complete rule constructors for exact whole-value replacement. This settles the first executable syntax without claiming arbitrary unknown-subterm extraction.

## Gates, scheduling, and state sharing

Retain incremental binding gates as an implementation of ordinary matching. Each gate belongs to a compatible joint witness and records concrete slot assignments. Use one agenda for new values, rule availability, completed bindings, view composition, and support changes. Gate completion interns an event; it does not destructively remove facts from a shared global database.

Both evaluators cache matching per target configuration/frame/pattern and index incoming/outgoing dependencies. Rust retains resumable candidate searches and incrementally delivers completed bindings to subscribed views, releasing temporary search state at completion. Distinct justifications update support for one binding. Retraction of support must not erase another justification. The completed-binding cache is not itself the support engine.

The deterministic coordinator optionally batches independent matching and normalization steps onto a Rayon pool. It merges results in queue order through idempotent state/event/support interning. Exact report comparisons across worker counts and continuation chunks cover the closed fixtures. Component-local well-founded support settles existing dependency SCCs, and unchanged snapshots reuse that result. This does not introduce early dynamic absence certificates or incremental program edits.

Canonicalization refines graph colors using sharing, capture, parent, and lexical links, then resumes exact candidate enumeration. Hash collisions must never merge different states. Include live rule environments, sharing, multiplicity, and continuations. Retain evidence separately so newly discovered support can awaken an existing state.

## Closure criteria

The accepted core implementation satisfies the following finite acceptance matrix. Arbitrary structural metaprogramming is outside this milestone. Broader claims of completeness or bounded resource usage still require their own argument and measurements.

| Obligation | Required observation |
| --- | --- |
| Generated rule, separate source | Seed.A infers Seed.B without copying R forward |
| Generated rule, shared source | Seed producing R.A infers B without resurrecting Seed |
| Alternative histories | Separate Seed → R and Seed → A do not enable B |
| Rule replacement | R.X becomes S.X; historical R.X and unrelated coherences retain their own rules |
| Local decoherence | Rule availability and every argument come from one eligible joint witness |
| Nested capture | Moving code preserves definition scope; invocation supplies a separate return continuation |
| Rule-derived abstraction | A whole rule matches an abstract description through ordinary source inference |
| Dynamic absence | A newly producible Q route reopens dependent absence without erasing independent support |
| Canonical identity | Equal structures/captures share; distinct captures or independent occurrences remain distinct |
| Fair gates | Arrival order and duplicate reports do not change the enabled event set |
| Source grammar | Returned rule values and invoked bodies have unambiguous lowering |

The dynamic examples cover these core interactions in the JavaScript reference and Rust runtime. Native lowering and CLI tests cover explicit rule values, bodies, multiple coherences, absence, and diagnostics. Unrestricted structural operations and strict resource isolation remain outside this finite implementation. Retained-record accounting is a soft batch-boundary limit, not a byte cap; graph setup and closure normalization remain non-preemptible. This supplies a concrete path to closing the gaps without claiming that a test matrix, physical analogy, or design preference proves a universally error-free language.
