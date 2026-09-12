# Configuration semantics

This contract describes the integrated finite [reference evaluator](kernel/model.js) and [interactive laboratory](plan.html). The [implementation plan](plan.md) records its delivery sequence, and the [binding contract](binding.md) summarizes projection. The Rust runtime implements the same bounded configuration contract, with [native source lowering](syntax.md) and a CLI. A separate generic Rust rule kernel provides literal multiset replacement. Both configuration evaluators execute finite ground generated rule values, lexical captures, read-supported source inference, and positive evidence.

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

Support is represented by positive clauses over states, events, and views. The initial state and identity views are facts. A composed view depends on its preceding view and event; an event depends on its source and joint witness; a resulting state depends on an incoming event. Rule availability is included in the witness and recorded separately from consumption.

Evidence closure starts from facts and adds a conclusion only when all its premises are established. Cycles without a base fact do not prove themselves. Duplicate arrivals cannot fill additional premise positions. Once established, evidence is never invalidated by later computation. There are no negative premises, absence queries, defaults, or conditional truth statuses.

`Not`, `True`, and `False` have no built-in meaning. A program may define `[True] False`, `[False] True`, or any other relation between its concepts. Molten does not judge those labels as logical assertions. The absence of an event never supplies a rule operand.

## Iteration and bounds

Use one fair agenda for candidate enumeration, view composition, and support propagation. These are implementation tasks within one iterative semantics, not language-level search and commitment phases. The JavaScript reference matcher retains compatible prefixes and per-slot candidates in an incremental gate for each joint witness configuration and frame. Out-of-order arrivals complete earlier prefixes; duplicate arrivals emit nothing. A persistent cache shares lazy binding enumeration across views of the same target, frame, and pattern. Rust retains resumable particle/gate searches under the same target/frame/pattern key and incrementally delivers cached bindings to subscribers. Optional Rayon workers advance independent search and canonicalization steps; the coordinator merges results in queue order. A gate counter is only a summary of its filled positions; duplicate reports cannot fill additional positions.

Intern configurations, events, views, and support clauses independently. A previously seen state may acquire new evidence and wake dependent applications. Do not restart its entire execution tree, and do not ignore genuinely new support. Cycles without new semantic information reach a fixed point. Programs producing unbounded distinct information may run forever.

The reference uses finite budgets for stored states, coherences, cells, frames, and agenda work. Rust also has a soft retained-record budget checked between coordinator batches; it can overshoot within a batch and does not count exact bytes. Exhaustion defers candidates. Increasing a budget resumes pending work. The intended production guarantee is fairness for every finite enabled derivation given sufficient resources. There is no general termination guarantee.

The accepted finite ground core is implemented. Rust canonicalization refines graph colors using sharing, captures, and frame links, then resumes exact candidate enumeration one step at a time. Positive support uses indexed premise counts and is cached across unchanged snapshots. These optimizations preserve the ground clause interpretation. Exact ordering remains worst-case factorial; graph setup, source compilation, and closure normalization are not strictly preemptible. No unrestricted scalability claim follows from the finite checks.

## Nested rule values

A ground value is an atom string or `{rule: {input, output}}`. A whole rule occupies one pattern position. The code interner normalizes particle/coherence order and ignores diagnostic names. Every active rule occurrence also carries a capture edge to its immutable defining frame. A pattern containing a whole rule resolves its expected capture in the pattern’s lexical environment. Equal code with a different capture is not silently an exact match.

Output `{particle: [{rule: ...}]}` returns a rule value; `{body: [...]}` enters a body. These intermediate-representation forms are explicit and executable. Native lowering uses `[Seed] [A] B` for produced rule values and `[Enter] ([Payload] Result)` for entered bodies. It reads the original grammar; see [generalization](generalization.md). The same lossless tree is used for executable lowering. The current constructors describe finite ground code, not arbitrary unknown subterm extraction or native wildcard substitution.

A rule exposed in a coherence becomes eligible there. The active rule index is consulted only in a compatible joint witness that also supplies its operands. Lexical declarations remain immutable within the model instance. Applying a local rule reads its availability; explicitly matching a rule as an operand consumes it by ordinary replacement. Applying a rule with an empty input vector is anchored to an existing execution-site coherence, just like an empty particle pattern.

An application records consuming footprint C and rule-read support R separately. Literal output replaces C and preserves R minus C as concrete remainder. Consumption wins on overlap. Rule availability and its derivation are retained in event support; output flow contains consuming dependencies rather than the union of all causal support.

For Seed → R, where R is A → B, actual execution from Seed.A reaches R.B. Inferred application at Seed.A produces Seed.B. If Seed jointly produces R.A, inferred application instead produces B: the source consumed through A is also the source that justified reading R. Separate competing Seed → R and Seed → A cannot provide a joint application.

Read support establishes an event at its historical source. Removing the rule later does not erase an already produced result. An inferred dynamic body retains the executable capture as frame metadata without allocating a consumable copy of a witness-only rule or adding read-only resources to its held consuming footprint.

Capture import preserves introduction sharing across all imported frames and between imported and already retained frames. Captured frames can themselves contain capture edges, including cycles. Canonicalization traverses that finite graph without recursively expanding it into a tree. The current equality includes captured frame structure and held introductions; production compaction must preserve the chosen observations before dropping metadata.

A whole rule can derive Function or another program-defined abstraction. Matching that description at its concrete source transfers the actual whole-rule witness according to the existing body convention. No separate type or meta matcher is introduced.

## Validation obligations

The integrated reference currently checks direct and inferred And, broadcast/reunion, sibling projection, incompatible alternatives, independent duplicate results, literal co-results, fresh production, three nested bodies, budget resumption, lexical capture, and canonical renaming invariance. These checks apply to its finite ground intermediate representation.

The dynamic fixtures cover local generated-rule execution, consuming whole-rule replacement, separate and shared read/consume sources, incompatible rule/data histories, multi-coherence activation, whole-rule abstractions, nested bodies, escaped captures, and later rule consumption. The Rust runtime, native lowering, and CLI have regression coverage alongside the reference suite. Arbitrary structural extraction and unbounded dynamic code constructors are outside the accepted core. Tests additionally compare exact reports across worker counts and continuation chunks, positive evidence closure against a simple fixed-point oracle, and suspension of large searches.

The laboratory contains the research comparison and primary source links. These semantics define the accepted implemented core; neither the tests nor the physical analogies establish universal correctness or completeness.
