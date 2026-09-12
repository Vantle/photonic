# Closing the runtime design

This records the accepted resolution of the remaining decisions in the [runtime plan](plan.html). The finite ground dynamic JavaScript reference now implements these core decisions: generated activation, read/consume projection, whole-rule replacement, capture, and conditional availability. Production optimizations and textual lowering remain explicit boundaries. Tests are not a proof of unrestricted correctness. The [configuration contract](semantics.md) continues to describe the tested finite evaluator.

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

The fixed compiled-rule label closure is disabled whenever rule values occur in the reference program. Dynamic queries conservatively remain open until supported evidence or finite completion resolves them. Early component-local certificates remain an optimization to implement. Keep negative cycles conditional rather than selecting a stable interpretation by evaluation order. SWI-Prolog's [well-founded semantics](https://www.swi-prolog.org/pldoc/man?section=WFS) and [incremental tabling](https://www.swi-prolog.org/pldoc/man?section=tabling-incremental) provide useful precedents; Molten still needs its own resource and source-projection contract.

## Surface syntax

Do not let parser convenience decide value semantics. Lower source into explicit atom, rule value, particle, coherence, and body forms. The existing punctuation should retain its purpose: dots combine within a particle, commas separate coherences, a bracketed input followed by an output forms a rule, and groups delimit expressions. Spaces distinguish expression boundaries where required; they are not runtime state.

There is a real ambiguity to resolve before freezing source lowering: an input-only bracket such as [B] does not yet specify whether it is a complete rule value with an empty result or merely a pattern expression, and a group of rules can denote a body rather than a returned code value. Consequently [[A,B]] ([B]) cannot be assigned executable behavior just from successful structural parsing.

Recommendation: require a complete rule value to carry an explicit output in the first executable source grammar. Reserve input-only brackets for pattern structure until an unambiguous constructor form is specified. Use a structured program representation in the reference to test whole-value replacement and activation first. Then choose the smallest surface notation that distinguishes returning a rule value from entering a body. No new runtime semantics is needed for that syntactic distinction.

This deliberately declines to bless the abbreviated example as executable code prematurely. The exact whole-value matching decision is settled; the printed abbreviation is not. A source specification must include positive and negative parsing/lowering examples for standalone brackets, nested rules, empty outputs, grouped bodies, and source whitespace before the frontend can promise execution.

## Gates, scheduling, and state sharing

Retain incremental binding gates as an implementation of ordinary matching. Each gate belongs to a compatible joint witness and records concrete slot assignments. Use one agenda for new values, rule availability, completed bindings, view composition, and support changes. Gate completion interns an event; it does not destructively remove facts from a shared global database.

The reference now retains a lazy binding cache per target configuration/frame/pattern and indexes incoming/outgoing dependencies. Production gates should additionally subscribe through indexes keyed by structural rule and pattern, scope, and source/view identity. Persist them across relevant updates rather than rebuilding them for every inspected view. Distinct justifications update support for one binding. Retraction of support must not erase another justification. The completed-binding cache is not itself the support engine.

Start with a deterministic single-threaded scheduler as the reference. Parallel workers can compute candidate deltas against immutable snapshots; merge them through idempotent state/event/support interning. Use semantic identities, not completion order, for deduplication. Fairly interleave resumable candidate enumeration and support tasks, including under repeated resource-budget suspension. Parallelization is an optimization after differential conformance tests pass.

Canonicalization may use hashing and graph refinement to find candidates efficiently, but confirm equality with the exact canonical structure. Hash collisions must never merge different states. Include live rule environments, sharing, multiplicity, and continuations. Retain evidence separately so newly discovered support can awaken an existing state.

## Closure criteria

A written recommendation closes an architectural question only provisionally. Mark it implemented when executable examples pass, and mark its broader properties established only with the corresponding argument or proof. Use the following acceptance matrix for the dynamic extension.

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

The dynamic examples now cover these core interactions together in the JavaScript reference. Textual grammar acceptance and unrestricted structural operations remain outside that finite representation; the Rust port remains the next implementation layer. This supplies a concrete path to closing the gaps without claiming that a test matrix, physical analogy, or design preference proves a universally error-free language.
