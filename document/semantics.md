# Configuration semantics

This contract describes the integrated finite [reference evaluator](kernel/model.js) and [interactive laboratory](plan.html). The [implementation plan](plan.md) records its delivery sequence, and the [binding contract](binding.md) summarizes projection. The Rust language runtime is not yet implemented. Exact nested rule-value replacement is supported by the generic Rust kernel. The JavaScript reference also executes finite ground generated rule values, lexical captures, read-supported source inference, and conditional availability.

## State

A configuration contains a multiset of coherences and their reachable lexical environments and continuations. A coherence contains a multiset of live occurrences. Each occurrence references an introduction identity and a concept value. Within a coherence, an introduction appears at most once; across coherences, the same introduction can be shared. Equal values with different introductions retain multiplicity.

Anonymous identity is not observable spelling. Canonicalization renames introductions and frames and orders coherences while preserving their incidence graph, lexical links, continuation links, and held resources. Equal labels alone do not establish equal configurations. An empty coherence differs from no coherence.

The reference fixes immutable lexical declarations and the finite ground code constructors for one model instance. Executable rule availability evolves with each configuration. Live rule values and their captured environments participate in canonical state identity. Code is structurally interned within the model, so caches cannot be shared across unrelated model instances without including their code universe.

## Witness

A relative view `V : S ⇒ T` records a joint derivation from a source configuration S to a witness configuration T. Its flow maps each live or continuation-held target occurrence to a set of source occurrences. Its context map tracks the participating source coherences, including empty ones. Its frame map tracks surviving source environments; newly created frames do not masquerade as earlier frames.

The identity view supplies direct matching. Composing a view with an event substitutes the event's flow through the earlier flow and interns the result. Different equivalent paths can support the same relative view. Provenance describes causal support, not exclusive global ownership.

Matching selects distinct occurrences for repeated particle positions and distinct witness coherences for distinct coherence positions. The reference requires participating coherences to share a frame. A local executable rule must reside in one of its participating coherences; an unrelated coherence cannot lend its code implicitly. Zero-input applications still bind an execution-site coherence, for both lexical declarations and local rule values. A joint sibling match may project several witness coherences to one concrete source coherence. This noninjective backward mapping does not duplicate the source.

Separate paths from incompatible alternatives are not a joint view. Given `A → B` and `A → C`, the historical presence of both results cannot enable a two-coherence B/C match. Given `A → [B,C]`, both results are present in one configuration and can enable that match.

## Application

Project the complete witness binding to its union of concrete live source occurrences, the footprint F. Preserve source coherences required by the context map, even when they contain no selected occurrence. Projection cannot borrow hidden continuation resources as additional live operands.

For literal output, remove introductions in F from the participating source coherences. Combine their remaining introductions by identity into remainder R. Each output receives R plus fresh introductions for its explicit result. Shared untouched remainder is reconciled once; independent equal introductions remain separate. Coherences outside the binding are unchanged.

For a body output, the exact footprint E consists of witness positions whose source mapping is a singleton occurrence carrying the same value. Consume E; transfer the remaining concrete witness and R into the new body. Store exact consumption in its continuation. The return destination and lexical definition environment are separate links. Applying a rule owned by the current body returns its output to the parent; applying an inherited rule operates in the current body.

A return associates explicit output with the body's consumed input and the enclosing held consumption. This allows aggregate inference to replace the entire original operator and operands. A local rule still preserves any unmatched operand: operator arity is described by program rules, not inferred from Boolean labels.

For `And.True.False.Extra`, deriving the witness `And.Boolean.Boolean.Extra` enables the outer body at the concrete source. Exact And is consumed, True.False.Extra transfers, and a body rule matching True.False produces False.Extra. Deriving an aggregate Boolean replaces And and both operands, preserving only Extra. Intermediate Boolean evidence never accumulates beside its concrete operands.

## Alternative source applications

Source inference is not observational shortcut elimination. With `A → B.C` and `B → D`, applying the latter at concrete A produces D. Sequentially executing both produces C.D. Keep both alternatives. Evidence-only co-results are not concrete source remainder.

Likewise, source inference can change the number of coherences relative to actually executing an evidence path. The joint footprint, concrete remainder, and final rule output determine the inferred result; the intermediate witness configuration is not copied forward. This is a Molten semantic choice, not a theorem of resource conservation or physics.

Every new explicit output gets an introduction distinct from sibling outputs. Independently computing inherited X to Y in two coherences yields two Y introductions. Computing Y once before divergence shares one Y introduction. Reunion must distinguish these cases even if their displayed labels agree.

## Event identity and support

The reference interns an event by its concrete source, frame, rule owner and identity, and projected binding. Its result is immutable. Distinct justifications extend support for that event without allocating another result. Interning a target state does not discard incoming events.

Support is represented by ground clauses over states, events, views, and scoped absence queries. The initial state and identity views are facts. A composed view depends on the earlier view and event; an event depends on its source, witness, and any absence condition; a resulting state depends on its incoming event.

An absence query is scoped to a source configuration, frame, and pattern. It asks whether an eligible joint derivation can match the pattern, not whether its labels happen to be missing now. Unfinished queries remain open. The fixed-rule reference can certify some impossible queries with a conservative label closure. Otherwise complete finite exploration provides closure. This certificate is not valid unchanged when runtime-generated rules enlarge the executable rule universe.

Well-founded support distinguishes supported, conditional, and unsupported derivations. Self-dependent absence remains represented even if there is no stable interpretation. An independent justification can preserve a state after another justification is defeated. These are support conditions, not a rejection of programs judged logically unreasonable.

The laboratory's revision comparison creates a new model with an added rule and preserves the previous snapshot. It is not an implementation of dynamic rule mutation or incremental revision maintenance.

## Iteration and bounds

Use one fair agenda for candidate enumeration, view composition, and support propagation. These are implementation tasks within one iterative semantics, not language-level search and commitment phases. The reference matcher retains compatible prefixes and per-slot candidates in an incremental gate for each joint witness configuration and frame. Out-of-order arrivals complete earlier prefixes; duplicate arrivals emit nothing. A persistent cache shares lazy binding enumeration across views of the same target, frame, and pattern. Particle-level matching still enumerates occurrences. Production work includes dependency maintenance across changing configuration content and parallel scheduling. A gate counter is only a summary of its filled positions; duplicate reports cannot fill additional positions.

Intern configurations, events, views, and support clauses independently. A previously seen state may acquire new evidence and wake dependent applications. Do not restart its entire execution tree, and do not ignore genuinely new support. Cycles without new semantic information reach a fixed point. Programs producing unbounded distinct information may run forever.

The reference uses finite budgets for stored states, coherences, cells, frames, and agenda work. Exhaustion defers candidates. Increasing a budget resumes pending work. The intended production guarantee is fairness for every finite enabled derivation given sufficient resources. There is no general termination or unrestricted absence decision guarantee.

The implementation is a finite executable specification. Canonicalization enumerates permutations in symmetric groups; support interpretation is computed from ground clauses. The reference now has adjacency indexes and persistent matching caches per target configuration, frame, and pattern. Production work still needs incremental graph refinement and bounded fine-grained task scheduling, compared against the reference. No production scalability claim follows from the finite checks.

## Nested rule values

A ground value is an atom string or `{rule: {input, output, negative?}}`. A whole rule occupies one pattern position. The code interner normalizes particle/coherence order and ignores diagnostic names. Every active rule occurrence also carries a capture edge to its immutable defining frame. A pattern containing a whole rule resolves its expected capture in the pattern’s lexical environment. Equal code with a different capture is not silently an exact match.

Output `{particle: [{rule: ...}]}` returns a rule value; `{body: [...]}` enters a body. These intermediate-representation forms are explicit and executable. The textual parser still needs lowering that distinguishes them; structural source parsing does not imply execution. The current constructors describe finite ground code, not arbitrary unknown subterm extraction or native wildcard substitution.

A rule exposed in a coherence becomes eligible there. The active rule index is consulted only in a compatible joint witness that also supplies its operands. Lexical declarations remain immutable within the model instance. Applying a local rule reads its availability; explicitly matching a rule as an operand consumes it by ordinary replacement. Applying a rule with an empty input vector is anchored to an existing execution-site coherence, just like an empty particle pattern.

An application records consuming footprint C and rule-read support R separately. Literal output replaces C and preserves R minus C as concrete remainder. Consumption wins on overlap. Rule availability and its conditional derivation are retained in event support; output flow contains consuming dependencies rather than the union of all causal support.

For Seed → R, where R is A → B, actual execution from Seed.A reaches R.B. Inferred application at Seed.A produces Seed.B. If Seed jointly produces R.A, inferred application instead produces B: the source consumed through A is also the source that justified reading R. Separate competing Seed → R and Seed → A cannot provide a joint application.

Read support establishes an event at its historical source. Removing the rule later does not erase an already produced result. Defeating an absence assumption can change its conditional support, which is a different operation. An inferred dynamic body retains the executable capture as frame metadata without allocating a consumable copy of a witness-only rule or adding read-only resources to its held consuming footprint.

Capture import preserves introduction sharing across all imported frames and between imported and already retained frames. Captured frames can themselves contain capture edges, including cycles. Canonicalization traverses that finite graph without recursively expanding it into a tree. The current equality includes captured frame structure and held introductions; production compaction must preserve the chosen observations before dropping metadata.

A whole rule can derive Function or another program-defined abstraction. Matching that description at its concrete source transfers the actual whole-rule witness according to the existing body convention. No separate type or meta matcher is introduced.

## Dynamic absence

When any rule value is present in the program representation, the fixed-rule label-closure shortcut is disabled. Queries remain open during generation and resolve through supported joint derivations or finite agenda completion. A generated route to Q can defeat a default based on absent Q, and conditional code availability remains a positive premise of every use.

This is conservative dependency handling: the reference does not issue a premature dynamic absence certificate that later code could invalidate. It does not yet implement component-local early certificates or incremental edits to an already closed model. The revision control creates a fresh immutable model and preserves the earlier snapshot. Unbounded code or value generation can leave absence conditional while other work proceeds.

## Validation obligations

The integrated reference currently checks direct and inferred And, broadcast/reunion, sibling projection, incompatible alternatives, independent duplicate results, literal co-results, fresh production, three nested bodies, conditional defaults, negative cycles, budget resumption, lexical capture, and canonical renaming invariance. These checks apply to its finite ground intermediate representation.

The dynamic fixtures cover local generated-rule execution, consuming whole-rule replacement, separate and shared read/consume sources, incompatible rule/data histories, multi-coherence activation, generated absence counterevidence, conditional code, whole-rule abstractions, nested bodies, escaped captures, and later rule consumption. Structural extraction, unbounded dynamic code constructors, textual lowering, and the Rust graph port remain separate implementation work.

The laboratory contains the research comparison and primary source links. These semantics are a concrete proposal with tested fragments; neither the tests nor the physical analogies establish universal correctness or completeness.
