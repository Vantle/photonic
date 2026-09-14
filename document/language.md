# Language

[Webbook](../index.html) · [Repository guide](../README.md)

- [Frontend contract](#syntax)
- [Configuration semantics](#semantics)
- [Replace or transfer the concrete binding](#binding)
- [Canonical states and coherence outputs](#state)
- [Generalization through rules](#generalization)
- [Shared-prefix groups](#group)
- [Terminology](#terminology)

<a id="syntax"></a>

## Frontend contract

Photonic source uses `.particle` for reusable rules and data, and `.wave` for scripts. Both extensions have identical grammar and evaluation; the distinction is a convention. JSON remains a separate structured interchange format.

Photonic has one grammar. The pest-backed `parser::parse` preserves the original source tree; `lowering::parse` interprets that same tree as an executable program. Lowering does not maintain another grammar or reserve additional characters or keywords.

The boundary follows the [original language document](https://github.com/Vantle/Vantle/blob/0b693aa583e71c60a225cbb3b4cd53ecfbbaf9fb/Molten/document/molten.page.rs): concepts, dots, commas, source contexts, groups, and ASCII whitespace. The nested-rule behavior below describes the current runtime; the historical constructor did not establish execution behavior for every form.

<a id="syntax-expressions"></a>

### Expressions

| Form | Example | Meaning |
| --- | --- | --- |
| Concept | `True` | One atom |
| Particle | `A.B` | Orderless occurrences in one coherence |
| Shared prefix | `A(B,C)` | Abbreviation for `A.B, A.C` |
| Coherences | `A, B` | Independent initial coherences |
| Rule | `[A] B` | Consume a matching source and produce B |
| Joint rule | `[A, B] (C, D)` | Join two inputs and produce two coherences |
| Scope | `[Enter] ([Payload] Result)` | Enter a body with a local rule |
| Produced rule | `[Seed] [A] B` | Produce the rule from A to B as one live value |
| Rule operand | `[[A] B] C` | Match a whole rule value and replace it with C |
| Initial rule value | `([A] B).A` | A live rule and an A occurrence |
| Empty particle | `()` | One empty coherence |
| Empty source | `[] A` | Zero operand positions, requiring an execution site |
| No result | `[A]` | No output coherences |
| Empty result | `[A] ()` | One output coherence, retaining unmatched remainder |

```text
And.True.False.Extra
[True] Boolean,
[False] Boolean,
[And.Boolean.Boolean] (
    [True.True] True,
    [True.False] False,
    [False.False] False,
)
```

Dots combine particle members; commas separate coherences. At module or body level, a source context starts a declaration. Commas before another declaration or at the end of a scope delimit declarations without creating an empty initial coherence. Explicit `()` creates an empty coherence. Whitespace separates expressions; it does not replace a dot inside an input particle.

A rule's following expression supplies its output. A following source context is itself a rule value: `[Seed] [A] B` constructs code. After a completed output, a new source context starts the next declaration. Use commas to terminate a rule with no output before another declaration: `[A], [B] C`.

An output group containing declarations enters their scope. A group of plain concepts only groups those concepts; it does not allocate a named record or introduce a frame. Multiple grouped destinations such as `[A] (B) (C)` produce separate coherences. A body can currently initialize at most one explicit coherence; this is a representation limitation diagnosed during lowering.

A nested context in a particle is a complete rule value, including the empty-output case. Thus `[[A,B]] ([B])` consumes the rule whose two inputs are A and B and whose output is empty, then enters a body containing the consuming rule `[B]`. It is not an alias declaration. For ordinary whole-rule replacement, write `[[A] B] [A] C`. See [generalization](#generalization).

<a id="syntax-no-additional-syntax"></a>

### No additional syntax

There are no variable sigils, quote operators, named constructors, arrows, semicolon terminators, braces, or absence keywords. The characters in `$x`, `@`, `->`, `;`, `{`, `}`, and the word `unless` are ordinary concept text wherever the original delimiter rules permit them. They have no special execution behavior. Old extended programs must be migrated; they are not interpreted by a compatibility grammar.

`A(B,C)` expands to `A.B,A.C` in initial states, rule patterns, plain outputs, and proof targets. Dot composition distributes: `(A,B).(C,D)` expands to four coherences. Repeated alternatives retain multiplicity. Initial occurrences are independent introductions, while inherited output remainders retain their shared identity. This spelling supplies no container, pairing identity, or whole-operand matching guard.

Packing is a separate [standard-library operation](library.md#structure). Load `//library:position` (including its application dependency) before invoking `Apply.Pack.([Position] 0).([Value] 2)` to construct `([0] 2)`. The frontend does not perform this computation. `Pack(Position.0, Value.2)` instead expands into two flat coherences and is not this operation. Literal fields may be written directly without invoking Pack.

`Box(A.B)` groups A and B alongside Box; it does not construct one opaque Box value. `$x` matches the literal concept `$x`, not an arbitrary value. `@([A] B)` includes an ordinary @ concept. Parentheses and brackets retain their original delimiter roles.

<a id="syntax-running-source"></a>

### Running source

```sh
bazel run //system:command -- run "$PWD/example/conjunction.wave"
bazel run //system:command -- run "$PWD/example/replacement.wave" --json
bazel run //system:command -- parse "$PWD/example/decoherence.wave"
bazel test //system:test
```

The CLI also accepts a structured JSON representation of the same positive program model. It has no negative-premise field, variable form, or named constructor form. Unknown fields are rejected rather than silently changing program meaning. `Not`, `True`, and `False` are ordinary concepts, not keywords or built-in logic.

<a id="syntax-representation-and-diagnostics"></a>

### Representation and diagnostics

The lossless tree borrows UTF-8 source and stores nodes in a flat vector with kinds, byte spans, and parent indices. Lowering builds child adjacency once and reads those nodes. Runtime values are atoms or complete rule values; there are no variable or named-structure variants, including through JSON.

Structural parsing preserves empty groups, repeated dots, and empty coherence positions. Executable lowering additionally checks expression composition. For example, `A..B` is structurally representable but has a missing operand. Missing or mismatched delimiters, unsupported body initialization, and excessive nesting have structured diagnostics. Delimiter nesting and executable nesting are each limited to 128 levels. Executable nesting also counts consecutive rule contexts, even when their brackets are shallow. These are implementation resource bounds. Shared-prefix distribution has a per-parse budget of one million units, counting generated structure and copied text bytes; excessive expansion returns `photonic::expansion` before allocation of that product. This prevents shallow expressions with many alternatives from bypassing depth protection.

Tests cover the original alphabet, Unicode spans, grouping, nested rule production and replacement, the historical bracket example, original-style Boolean declarations, empty results, malformed composition, and runtime reachability. They do not establish that every previously unresolved historical expression has a unique intended semantics.

<a id="semantics"></a>

## Configuration semantics

This contract describes the integrated finite [reference evaluator](kernel/model.js) and [interactive laboratory](reference.html). The [runtime guide](runtime.md#runtime) describes its implementation, and the [binding contract](#binding) summarizes projection. The Rust runtime implements the same bounded configuration contract, with [native source lowering](#syntax) and a CLI. A separate generic Rust rule kernel provides literal multiset replacement. Both configuration evaluators execute finite ground generated rule values, lexical captures, read-supported source inference, and positive evidence.

<a id="semantics-state"></a>

### State

A configuration contains a multiset of coherences and their reachable lexical environments and continuations. A coherence contains a multiset of live occurrences. Each occurrence references an introduction identity and a concept value. Within a coherence, an introduction appears at most once; across coherences, the same introduction can be shared. Equal values with different introductions retain multiplicity.

Anonymous identity is not observable spelling. Canonicalization renames introductions and frames and orders coherences while preserving their incidence graph, lexical links, continuation links, and held resources. Equal labels alone do not establish equal configurations. An empty coherence differs from no coherence.

The reference fixes immutable lexical declarations and the finite ground code constructors for one model instance. Executable rule availability evolves with each configuration. Live rule values and their captured environments participate in canonical state identity. Code is structurally interned within the model, so caches cannot be shared across unrelated model instances without including their code universe.

<a id="semantics-witness"></a>

### Witness

A relative view `V : S ⇒ T` records a joint derivation from a source configuration S to a witness configuration T. Its flow maps each live or continuation-held target occurrence to a set of source occurrences. Its context map tracks the participating source coherences, including empty ones. Its frame map tracks surviving source environments; newly created frames do not masquerade as earlier frames.

The identity view supplies direct matching. Composing a view with an event substitutes the event's flow through the earlier flow and interns the result. Different equivalent paths can support the same relative view. Provenance describes causal support, not exclusive global ownership.

Matching selects distinct occurrences for repeated particle positions and distinct witness coherences for distinct coherence positions. The reference requires participating coherences to share a frame. A local executable rule must reside in one of its participating coherences; an unrelated coherence cannot lend its code implicitly. Zero-input applications still bind an execution-site coherence, for both lexical declarations and local rule values. A joint sibling match may project several witness coherences to one concrete source coherence. This noninjective backward mapping does not duplicate the source.

Separate paths from incompatible alternatives are not a joint view. Given `A → B` and `A → C`, the historical presence of both results cannot enable a two-coherence B/C match. Given `A → [B,C]`, both results are present in one configuration and can enable that match.

<a id="semantics-application"></a>

### Application

Project the complete witness binding to its union of concrete live source occurrences, the footprint F. Preserve source coherences required by the context map, even when they contain no selected occurrence. Projection cannot borrow hidden continuation resources as additional live operands.

For literal output, remove introductions in F from the participating source coherences. Combine their remaining introductions by identity into remainder R. Each output receives R plus fresh introductions for its explicit result. Shared untouched remainder is reconciled once; independent equal introductions remain separate. Coherences outside the binding are unchanged.

For a body output, the exact footprint E consists of witness positions whose source mapping is a singleton occurrence carrying the same value. Consume E; transfer the remaining concrete witness and R into the new body. Store exact consumption in its continuation. The return destination and lexical definition environment are separate links. Applying a rule owned by the current body returns its output to the parent; applying an inherited rule operates in the current body.

A return associates explicit output with the body's consumed input and the enclosing held consumption. This allows aggregate inference to replace the entire original operator and operands. A local rule still preserves any unmatched operand: operator arity is described by program rules, not inferred from Boolean labels.

For `And.True.False.Extra`, deriving the witness `And.Boolean.Boolean.Extra` enables the outer body at the concrete source. Exact And is consumed, True.False.Extra transfers, and a body rule matching True.False produces False.Extra. Deriving an aggregate Boolean replaces And and both operands, preserving only Extra. Intermediate Boolean evidence never accumulates beside its concrete operands.

<a id="semantics-alternative-source-applications"></a>

### Alternative source applications

Source inference is not observational shortcut elimination. With `A → B.C` and `B → D`, applying the latter at concrete A produces D. Sequentially executing both produces C.D. Keep both alternatives. Evidence-only co-results are not concrete source remainder.

Likewise, source inference can change the number of coherences relative to actually executing an evidence path. The joint footprint, concrete remainder, and final rule output determine the inferred result; the intermediate witness configuration is not copied forward. This is a Photonic semantic choice, not a theorem of resource conservation or physics.

Every new explicit output gets an introduction distinct from sibling outputs. Independently computing inherited X to Y in two coherences yields two Y introductions. Computing Y once before divergence shares one Y introduction. Reunion must distinguish these cases even if their displayed labels agree.

<a id="semantics-event-identity-and-support"></a>

### Event identity and support

The reference interns an event by its concrete source, frame, rule owner and identity, and projected binding. Its result is immutable. Distinct justifications extend support for that event without allocating another result. Interning a target state does not discard incoming events.

Support is represented by positive clauses over states, events, and views. The initial state and identity views are facts. A composed view depends on its preceding view and event; an event depends on its source and joint witness; a resulting state depends on an incoming event. Rule availability is included in the witness and recorded separately from consumption.

Evidence closure starts from facts and adds a conclusion only when all its premises are established. Cycles without a base fact do not prove themselves. Duplicate arrivals cannot fill additional premise positions. Once established, evidence is never invalidated by later computation. There are no negative premises, absence queries, defaults, or conditional truth statuses.

`Not`, `True`, and `False` have no built-in meaning. A program may define `[True] False`, `[False] True`, or any other relation between its concepts. Photonic does not judge those labels as logical assertions. The absence of an event never supplies a rule operand.

<a id="semantics-iteration-and-bounds"></a>

### Iteration and bounds

Use one fair agenda for candidate enumeration, view composition, and support propagation. These are implementation tasks within one iterative semantics, not language-level search and commitment phases. The JavaScript reference matcher retains compatible prefixes and per-slot candidates in an incremental gate for each joint witness configuration and frame. Out-of-order arrivals complete earlier prefixes; duplicate arrivals emit nothing. A persistent cache shares lazy binding enumeration across views of the same target, frame, and pattern. Rust retains resumable particle/gate searches under the same target/frame/pattern key and incrementally delivers cached bindings to subscribers. Optional Rayon workers advance independent search and canonicalization steps; the coordinator merges results in queue order. A gate counter is only a summary of its filled positions; duplicate reports cannot fill additional positions.

Intern configurations, events, views, and support clauses independently. A previously seen state may acquire new evidence and wake dependent applications. Do not restart its entire execution tree, and retain each distinct supporting derivation. Cycles without new semantic information reach a fixed point. Programs producing unbounded distinct information may run forever.

The reference uses finite budgets for stored states, coherences, cells, frames, and agenda work. Rust also has a soft retained-record budget checked between coordinator batches; it can overshoot within a batch and does not count exact bytes. Exhaustion defers candidates. Increasing a budget resumes pending work. The intended production guarantee is fairness for every finite enabled derivation given sufficient resources. There is no general termination guarantee.

The accepted finite ground core is implemented. Rust canonicalization refines graph colors using sharing, captures, and frame links, then resumes exact candidate enumeration one step at a time. Positive support uses indexed premise counts and is cached across unchanged snapshots. These optimizations preserve the ground clause interpretation. Exact ordering remains worst-case factorial; graph setup, source compilation, and closure normalization are not strictly preemptible. No unrestricted scalability claim follows from the finite checks.

<a id="semantics-nested-rule-values"></a>

### Nested rule values

A ground value is an atom string or `{rule: {input, output}}`. A whole rule occupies one pattern position. The code interner normalizes particle/coherence order and ignores diagnostic names. Every active rule occurrence also carries a capture edge to its immutable defining frame. A pattern containing a whole rule resolves its expected capture in the pattern’s lexical environment. Equal code with a different capture is not silently an exact match.

Output `{particle: [{rule: ...}]}` returns a rule value; `{body: [...]}` enters a body. These intermediate-representation forms are explicit and executable. Native lowering uses `[Seed] [A] B` for produced rule values and `[Enter] ([Payload] Result)` for entered bodies. It reads the original grammar; see [generalization](#generalization). The same lossless tree is used for executable lowering. The current constructors describe finite ground code, not arbitrary unknown subterm extraction or native wildcard substitution.

A rule exposed in a coherence becomes eligible there. The active rule index is consulted only in a compatible joint witness that also supplies its operands. Lexical declarations remain immutable within the model instance. Applying a local rule reads its availability; explicitly matching a rule as an operand consumes it by ordinary replacement. Applying a rule with an empty input vector is anchored to an existing execution-site coherence, just like an empty particle pattern.

An application records consuming footprint C and rule-read support R separately. Literal output replaces C and preserves R minus C as concrete remainder. Consumption wins on overlap. Rule availability and its derivation are retained in event support; output flow contains consuming dependencies rather than the union of all causal support.

For Seed → R, where R is A → B, actual execution from Seed.A reaches R.B. Inferred application at Seed.A produces Seed.B. If Seed jointly produces R.A, inferred application instead produces B: the source consumed through A is also the source that justified reading R. Separate competing Seed → R and Seed → A cannot provide a joint application.

Read support establishes an event at its historical source. Removing the rule later does not erase an already produced result. An inferred dynamic body retains the executable capture as frame metadata without allocating a consumable copy of a witness-only rule or adding read-only resources to its held consuming footprint.

Capture import preserves introduction sharing across all imported frames and between imported and already retained frames. Captured frames can themselves contain capture edges, including cycles. Canonicalization traverses that finite graph without recursively expanding it into a tree. The current equality includes captured frame structure and held introductions; production compaction must preserve the chosen observations before dropping metadata.

A whole rule can derive Function or another program-defined abstraction. Matching that description at its concrete source transfers the actual whole-rule witness according to the existing body convention. No separate type or meta matcher is introduced.

<a id="semantics-validation-obligations"></a>

### Validation obligations

The integrated reference currently checks direct and inferred And, broadcast/reunion, sibling projection, incompatible alternatives, independent duplicate results, literal co-results, fresh production, three nested bodies, budget resumption, lexical capture, and canonical renaming invariance. These checks apply to its finite ground intermediate representation.

The dynamic fixtures cover local generated-rule execution, consuming whole-rule replacement, separate and shared read/consume sources, incompatible rule/data histories, multi-coherence activation, whole-rule abstractions, nested bodies, escaped captures, and later rule consumption. The Rust runtime, native lowering, and CLI have regression coverage alongside the reference suite. Arbitrary structural extraction and unbounded dynamic code constructors are outside the accepted core. Tests also compare exact reports across worker counts and continuation chunks, positive evidence closure against a simple fixed-point oracle, and suspension of large searches.

The laboratory contains the research comparison and primary source links. These semantics define the accepted implemented core; neither the tests nor the physical analogies establish universal correctness or completeness.

<a id="binding"></a>

## Replace or transfer the concrete binding

The [configuration semantics](#semantics) defines how bindings behave. The Rust runtime and bounded JavaScript [reference](reference.html) integrate source projection with multiple coherences, multiple outputs, nested bodies, fresh introductions, captured rule values, and positive evidence. This document summarizes their binding contract.

<a id="binding-concrete-source-projection"></a>

### Concrete source projection

A match selects occurrences in one joint witness configuration. A relative view maps those occurrences back to their concrete source footprint and tracks the source coherences and environments involved. Several witness outputs may project to one source occurrence. The source is consumed once; independently substituting an ancestor for every pattern position would lose this relationship.

For a literal result, remove the projected introductions from participating source coherences. Reconcile their unmatched remainder by introduction identity, then give each output that remainder and fresh introductions for its explicit result. Unselected coherences are unchanged.

For a nested body, exact source matches are consumed and held by its continuation. Transfer the other concrete witness resources and the unrelated remainder into the body. The body uses the same application mechanism. Unmatched body resources survive return; no Boolean label or operand position has a special disposal rule.

Lexical definition and return destination are separate frame links. A returned explicit result depends on its body's consumed inputs and held enclosing consumption. This makes an aggregate result project to the operator and operands it actually used.

<a id="binding-examples"></a>

### Examples

| Application at its concrete source | Result |
| --- | --- |
| `True → Boolean` at `True.Extra` | `Boolean.Extra` |
| `B → C` inferred at `A.Extra` through `A → B` | `C.Extra` |
| `B → A` inferred at `A` through `A → B` | `A` |
| `Not.Boolean → body` at `Not.True.Extra` | The body receives `True.Extra`; exact `Not` is held |
| `Not.True → body` at `Not.True.Extra` | The body receives `Extra`; exact `Not.True` is held |
| `Not.Boolean → False` at `Not.True.Extra` | `False.Extra` |
| `[B, C] → D` inferred through `A → [B, C]` at `A.X` | `D.X`, with A consumed once |

For explicit And, the witness `And.Boolean.Boolean.Extra` enables a body at `And.True.False.Extra`. The body receives `True.False.Extra`; `True.False → False` produces `False.Extra`. An aggregate Boolean inference consumes And and both operands, preserving Extra. Intermediate Boolean deductions are evidence, not additional live operands.

<a id="binding-evidence-only-outputs"></a>

### Evidence-only outputs

With `A → B.C` and `B → D`, source inference at A produces D. Executing the producer and then the second rule produces C.D. Both alternatives remain in the graph. Source inference is a semantic operation, not an optimization that may replace an equivalent sequence.

Concrete source remainder survives an inferred application. Results produced only along an evidence path do not automatically become that remainder. Unrelated intermediate computation also does not enlarge the consumed footprint merely because it appears along an evidence path.

<a id="binding-identity-and-support"></a>

### Identity and support

Two unchanged inherited X references reconcile once. Two independently produced Ys remain independently usable, including when their computations share causal ancestry. Source-flow dependencies therefore cannot serve as a global exclusive ownership key.

Applications retain evidence for the source, witness, and rule availability. Equal configurations share storage while all distinct incoming evidence remains represented. Evidence is monotone: later consumption changes a successor state, not the validity of earlier events.

Whole rule-value replacement follows ordinary consuming replacement in the Rust generic kernel, Rust runtime, and JavaScript reference. The two configuration evaluators also activate finite ground rule values present in participating coherences. A definition is read when invoked and consumed when explicitly matched as an operand.

For generated code, read support and consumed operands are separate. If Seed produces `R: A → B`, sequential execution from Seed.A can reach R.B, while inferred application at Seed.A produces Seed.B. Seed supplied code; only A was consumed. If one Seed jointly produces R and A, the inferred application consumes that Seed through its operand projection and produces B.

The rule and its arguments must coexist in one witness. Alternative histories supplying code and data cannot be combined. A read dependency enables an event at its source; later consumption of the code does not erase an already produced result.

A body retains the invoked definition's lexical capture without materializing a new rule operand. Captured environments can escape their defining body, and imports preserve introductions shared by their held occurrences. Read support must not enter the body's held consuming footprint.

The implemented higher-order representation uses finite ground constructors and exact whole values. Abstract descriptions can be reached through ordinary derivations. The [native frontend](#syntax) lowers explicit whole-rule constructors and bodies. Arbitrary partial structural matching and behavioral equivalence are outside the accepted core.

See [configuration semantics](#semantics) for the normative reference details and [native library guide](library.md#remaining) for remaining construction work.

<a id="state"></a>

## Canonical states and coherence outputs

The [configuration semantics](#semantics) is the current contract. The Rust runtime and [JavaScript laboratory](reference.html) integrate configuration sharing, joint source projection, resource allocation, captured rule values, nested frames, and positive evidence closure. Earlier local-state and tuple experiments remain historical references.

<a id="state-share-computation-preserve-structure"></a>

### Share computation, preserve structure

State content is orderless: A.B equals B.A, while A.A retains independent multiplicity. A configuration contains coherences and their reachable frames. An empty coherence differs from no coherence.

Canonicalization renames anonymous introductions and frames while preserving their incidence graph: which occurrences share an introduction, which coherence holds each occurrence, and which frames define lexical scope, return destinations, and held resources. Equal labels alone are insufficient. Two equal explicit output coherences remain two positions even when their contents share storage.

The reference interns finite ground rule structures into code keys within one model instance. Live rule values and their captured frames participate in configuration identity. Canonicalization follows capture edges from live and held values, preserves their sharing, and retains reachable frames even after a closure escapes. It handles cyclic capture graphs; equal code with distinct captured structure does not collapse into one executable value.

Support remains in clauses over states, events, views, and queries; sharing a state does not discard its alternative conditions. Code keys are model-local, so caches are not reused across unrelated compiled programs.

A → A may close as a self-loop. A → B → A can return to a shared configuration. These graph cycles do not place an old source beside its successor as a second live input.

<a id="state-shared-remainder-and-fresh-results"></a>

### Shared remainder and fresh results

For the atomic rule `[A, B] → [C, D]`:

```
[A.X, B.Y] → [C.X.Y, D.X.Y]
[C, D] → E
Result: E.X.Y
```

Both outputs carry references to the same X and Y introductions, so reunion preserves each once. Independently introduced equal Xs remain X.X. Removal applies across the complete participating binding: consuming an X alias prevents another participating alias from restoring it. Unselected coherences are not mutated.

Explicit outputs receive fresh introductions. Therefore:

| Branch behavior before reunion | Carried result |
| --- | --- |
| Both carry the same inherited X unchanged | X |
| One computes X → Y, the other X → Z | Y.Z |
| Both independently compute X → Y | Y.Y |
| Y is computed once before divergence | One shared Y |

An explicit X → X is also production. In an isolated coherence it can be alpha-equivalent to its input; alongside another coherence retaining the original X, the changed sharing pattern distinguishes the result. Causal ancestry is not a global exclusive consumption permission.

<a id="state-joint-evidence"></a>

### Joint evidence

Match within a complete reachable configuration. A Cartesian product of historical states can combine incompatible alternatives and is not a witness.

If A jointly produces B and C, `[B, C] → D` can project back to A once. If `A → B` and `A → C` are competing alternatives, there is no joint B/C configuration. The same ancestor label does not decide these cases; the producing event and its relative flow do.

Source inference also preserves its own output law. With `A → B.C` and `B → D`, inferred D and sequential C.D remain distinct alternatives. Evidence-only co-results are not copied into a concrete source result automatically.

<a id="state-new-evidence-and-matching-gates"></a>

### New evidence and matching gates

Intern configurations, events, relative views, and support clauses independently. A new view can enable an application at an already known source. Equivalent evidence extends support for an existing application instead of allocating another result. Only positively established evidence enables computation; a cycle cannot establish its own missing premises.

Both evaluators cache matching by target configuration, frame, and pattern. Both retain lazy enumeration progress; Rust incrementally delivers completed cached bindings to subscribers and releases the candidate search when finished. Compatible occurrence assignments retain their identities; repeated reports do not supply another operand. Different relative views can reuse those cached bindings while projecting their own source footprints. Adjacency indexes schedule related work without pooling incompatible configurations. Rust can advance independent search and normalization steps on a Rayon pool, then merge their results in coordinator queue order.

Generated definitions activate locally and retain read support separately from consuming flow. Established historical evidence is preserved when later events consume or replace code. There is no absence-dependent invalidation.

<a id="state-growth-and-bounds"></a>

### Growth and bounds

Unchanged inherited context can close a divergence/reunion cycle:

```
A.X → [B.X, C.X] → A.X
```

Independent branch production can instead create genuinely larger configurations:

```
A.X → [B.X, C.X]
both branches compute X → Y
[B, C] → A
A.Y.Y
both Ys compute Y → X
A.X.X
```

The larger multiplicity is observable and cannot be erased by memoization. Writing rules in both directions does not make them inverses. Budgets suspend unfinished work; they do not establish a semantic depth limit.

Initial configurations contain independently introduced occurrences, so Rust sorts their coherences and renames once without factorial initial-world enumeration. General configurations refine graph colors using sharing, capture, parent, and lexical edges before exact canonicalization. Its resumable search advances one ordering/candidate at a time and reuses completed normalization slots. Worst-case CPU work remains factorial. Matching caches and adjacency indexes reduce repeated work, but finite checks of identity, sharing, scope, projection, and support do not establish production performance or universal completeness. The Rust graph runtime and executable textual lowering are implemented for this finite ground fragment. The accepted core includes resumable particle matching and candidate ordering. Arbitrary partial structural operations are outside this milestone; graph setup, source compilation, and closure normalization are not strictly preemptible. The integrated suite exercises finite ground dynamic rules, capture identity, support, and cache behavior.

The soft retained-record budget is checked between coordinator batches and can overshoot within a batch. Snapshots report current and peak counts; they do not measure exact bytes or provide reloadable checkpoints. Support is solved by dependency component and cached for unchanged snapshots. See [runtime](runtime.md#runtime) for the execution controls.

Native execution uses the [original grammar](#syntax). Whole-rule generalization is documented [here](#generalization); arbitrary structural substitution is not implemented. The language has no negative premises.

<a id="generalization"></a>

## Generalization through rules

Use the same source expression at every level. A concept can derive an abstraction; a whole rule can derive an abstraction; either can be consumed or carried into a body by the existing projection contract. There is no special grammar for types, quotation, or metavariables.

<a id="generalization-abstraction-preserves-its-concrete-witness"></a>

### Abstraction preserves its concrete witness

```text
Not.True
[True] Boolean
[False] Boolean
[Not.Boolean] (
    [True] False,
    [False] True,
)
```

The derivation of Boolean enables the Not rule at the concrete source. Its body receives True, and its local rule returns False. Boolean is evidence for this application, not an extra operand carried alongside True. The same mechanism applies when the concrete witness is a whole rule value.

<a id="generalization-repeated-abstractions-accept-different-concrete-operands"></a>

### Repeated abstractions accept different concrete operands

```text
Pair.Seed,
Pair.Other
[Seed] Intermediate
[Intermediate] Kind
[Other] Kind
[Pair.Kind, Pair.Kind] ([] Result)
```

This executable example reaches Result.Seed.Other. The two Kind occurrences are ordinary literal patterns, enabled by the program's derivations. Seed and Other need not be equal. Their evidence can require different numbers of steps. The joint application can open its body at the concrete source, preserving Seed and Other; the local rule adds Result and returns. Removing Other's derivation prevents this application. Putting both operands in one coherence also prevents this two-coherence match.

`system/test/obsidian.rs` checks these claims, including operand permutation. This example demonstrates abstraction and concrete witness preservation across flat coherences. It does not implement nested groups, copy either operand, or compute multiplication.

The intended `Multiply(Number,Number)` abbreviates `Multiply.Number, Multiply.Number`; it does not introduce a private operand container. There are no implicit variables, no requirement that both numbers be equal, and no builtin numerical type. The distinction between partial pattern matching and evidence for a complete operand is recorded in [shared-prefix groups](#group).

<a id="generalization-code-is-another-value"></a>

### Code is another value

```text
Seed.A
[Seed] [A] B
```

Seed produces the rule from A to B. That rule is locally executable where it is present. Invoking it reads its availability; matching it as an operand consumes it. Actual execution reaches the live rule alongside B; source inference also enables Seed.B. Both alternatives retain their existing support and ownership semantics.

```text
([A] B).A
[[A] B] [A] C
```

The outer rule replaces the complete A-to-B rule with an A-to-C rule. The produced code executes through the ordinary runtime. This is local replacement, not global equivalence or an alias that retroactively changes earlier events. Whole-rule matching compares normalized code and its lexical capture under the ground matching contract.

<a id="generalization-what-this-establishes"></a>

### What this establishes

Nested brackets express rule values without an @ operator. Ordinary derivations express abstraction without a $ variable. Groups express scoped rule bodies without braces. All three use the existing matcher, source projection, event support, and canonical configuration machinery.

This is deliberately smaller than arbitrary structural substitution. It does not provide a generic operation that opens any unknown rule, binds its input fields, and rebuilds different code. The former variable/constructor extension and its implementation have been removed, not renamed or hidden in JSON. A rule can carry unknown concrete information into a body through source inference and remainder transfer; that does not automatically give it arbitrary structural field access.

The finite ground code shapes come from the program. Runtime activation, replacement, and captured environments evolve, but this implementation does not synthesize arbitrary new code shapes from metavariables. A finite code catalog does not guarantee a finite state graph: multiplicity, coherences, and retained captures can still grow.

<a id="generalization-remaining-research"></a>

### Remaining research

Operand abstraction comes from ordinary rule derivations. There are no explicit capture forms, variable conventions, or hidden binding operations in the punctuation. `Multiply(Number,Number)` abbreviates two flat coherences. Each Number occurrence requires evidence supplied by rules.

Establish encodings of binding, reusable hypotheses, and induction using rule composition before claiming a universal mathematical foundation. If an operation cannot yet be encoded, record that gap. Do not introduce an implicit wildcard convention, a reserved concept masquerading as an ordinary label, or a theorem-specific evaluator.

All rules require positive evidence. Negative premises are not a missing feature or a future encoding target: the language does not define them. A user-defined concept named Not is allowed, with exactly the behavior its program supplies. Meta rules and abstractions can compress explicit definitions; they do not create default behavior from missing evidence.

The [natural-number examples](arithmetic.md#natural) demonstrate a useful immediate simplification: unary quantities use only Unit multiplicity, with empty as zero; addition uses ordinary coherence reunion. They require no record constructors or pattern variables.

The [shared-prefix investigation](#group) records the implemented elaboration of original expressions such as `A(B,C)` into flat coherences. Whole-operand guards within ordinary rules remain unresolved; a [retained-input multiplication construction](arithmetic.md#product) supplies both counts as data, with bounded validation and explicit remaining proof obligations; shared spelling alone supplies neither.

<a id="group"></a>

## Shared-prefix groups

The frontend expands `A(B,C)` into `A.B, A.C`, saving you from writing the prefix twice. The result is flat: it has no private container or pairing identity. Use the [library packing protocol](library.md#structure) when a computation needs to construct an indexed field.

<a id="group-one-flat-construction"></a>

### One flat construction

`Multiply(Number,Number)` expands to `Multiply.Number, Multiply.Number`. Number remains an ordinary literal concept. Each occurrence requires evidence from program rules; neither is a variable, and the concrete witnesses need not be equal. Existing source inference can apply a rule at the concrete source of that evidence.

The abbreviation must elaborate consistently in initial configurations, patterns, and plain outputs. It must preserve the behavior of the fully written form, including empty groups, repeated occurrences, nested parentheses, and depth diagnostics. Scoped rule bodies retain their existing interpretation. Dot composition distributes over alternatives: `(A,B).(C,D)` expands to `A.C,A.D,B.C,B.D`. Parenthesized alternatives can nest; `()` contributes one empty particle. In a rule destination, adjacent destination groups remain separate outputs, as before: `[Seed] (A)(B)` emits A and B in distinct coherences. A one-million-unit frontend budget bounds generated structure and copied text; exhaustion returns a structured expansion diagnostic before materialization.

Repeated source text and runtime broadcasting have different provenance. Two separately written initial occurrences are independent introductions. A runtime split can carry one inherited occurrence into multiple coherences, and reunion reconciles that shared occurrence once. A textual abbreviation must not silently turn independent introductions into a shared resource.

<a id="group-many-coherences"></a>

### Many coherences

A two-input rule selects two distinct eligible coherences in an applicable execution context. With four independently introduced, distinguishable operands carrying Multiply, every eligible unordered pair is a possible application. Source adjacency supplies no pairing information. Permutation-equivalent applications and states can be deduplicated; semantically different pairings remain different possibilities.

These applications are alternative successor events, not a promise to execute every overlapping pair together. Each event follows the usual consumption and provenance rules. Independent work can execute in parallel; overlapping resource use must retain the existing event semantics.

If a program requires two particular calculations to remain separate, it must supply that distinction through its existing execution contexts or ordinary labels and rules. Merely assigning the same Multiply atom to all four operands cannot communicate two intended pairs. Introducing hidden pair identifiers would add semantics beyond the abbreviation.

<a id="group-whole-operand-evidence"></a>

### Whole-operand evidence

The preferred numerical contract is that the complete operand establishes Number. `Unit.Extra` deriving `Number.Extra` does not establish that the entire operand is Number. Conversely, a program may explicitly define a derivation using both Unit and Extra to establish Number; the runtime should not reserve or reject Extra.

Exact complete-state evidence is available through Obsidian, and executable examples demonstrate it. This preference is not an automatic guard in the flat pattern matcher. Ordinary matching is open: a pattern can select part of a coherence, and unmatched contents follow the existing remainder law. Thus `[Multiply.Number, Multiply.Number]` alone does not certify that Number accounts for every other occurrence in each coherence.

Do not silently change every rule to exact whole-coherence matching. That would change existing remainder transfer and composition. Also do not specialize Number or Multiply in the runtime. The remaining question is how a rule can request a complete operand boundary while preserving ordinary open matching elsewhere. Any successful open match still succeeds with an unrelated inert Extra in its coherence; another positive open rule cannot remove that fact. If it requires a general boundary or evidence contract, specify that change explicitly before implementation. Parentheses used only as shorthand do not supply such a boundary.

<a id="group-multiplication-acceptance"></a>

### Multiplication acceptance

A grouped spelling does not implement multiplication. A single rule `[Multiply.Unit, Multiply.Unit] Unit` consumes two units, emits one, and reunites the remaining units: for positive independent inputs it produces m + n − 1, not their product.

Completion requires one fixed ordinary-rule program taking two runtime numerals, with zero, one, operand permutations, larger inputs, and incorrect pure-numeral targets checked. A coefficient embedded in an output action is still fixed-action scaling. The rejected preparation experiment in [multiplication](verification.md#counterexample) demonstrates why a completion marker cannot certify that all input units were processed.

No arithmetic primitive, implicit capture, negative premise, or rule priority is part of this direction. A [retained-input two-numeral construction](arithmetic.md#product) has executable evidence. It requires helper rule values and an exact independent-product target; the bare grouped multiplier and general whole-operand guard remain unfinished. See the [executable walkthrough](verification.md#group) for the implemented behavior and the precise remaining boundary.

<a id="terminology"></a>

## Terminology

Rules describe computation, including abstraction derivations. Events record applications. The [configuration semantics](#semantics) defines their current reference behavior; the [core design](#semantics) summarizes the direction.

| Term | Meaning |
| --- | --- |
| Concept | A value carried by an occurrence, including an atom such as `Ready` or a whole ground rule value. |
| Particle | A multiset of live occurrences within one coherence. Independent equal values retain multiplicity. |
| Coherence | An independently evolving parallel context. |
| Configuration | Jointly available coherences with their reachable lexical environments and continuations. |
| Rule | Describes input matching and output behavior; a whole rule can also be a value matched by another rule. |
| Closure | An executable rule value with its captured lexical environment. |
| Read support | Evidence enabling access to a definition without consuming its source as an operand. |
| Event | An application at a concrete source configuration, with its binding, output flow, and support. |
| Introduction | Identity created for an explicit result, preserved when that result is carried unchanged. |
| Occurrence | A local reference to an introduction within a coherence or held continuation. |
| Relative view | A joint derivation with flow from target occurrences to concrete source occurrences. |
| Footprint | Concrete source occurrences selected by projecting a witness binding. |
| Remainder | Unmatched concrete source resources carried to an application's outputs. |
| Frame | A lexical environment and continuation record with separate definition and return links. |
| Support | Positive evidence establishing a state, event, or view; conclusions require all their premises. |
| Record | A retained runtime entry counted by the soft exploration budget; it is not an exact byte measure. |
| Executor | An optional worker pool for independent implementation steps; the coordinator merges their results in deterministic queue order. |
| Gate | Compatible partial slot bindings within one joint configuration and frame. Cached match enumeration can reuse them across relative views. |

Use **input**, **output**, and **apply** for rule operations. **Divergence** creates several coherences; **decoherence** combines compatible coherences. These describe ordinary rule behavior. Multiset union remains **merge**.

**Canonical state** means shared configuration content with anonymous identity normalized while preserving multiplicity and sharing. **History** means the event record; **version** may identify a historical coherence occurrence. A repeated canonical state is not a second live copy of its history. **Origin**, when used in older experiments, refers to introduction or dependency information; it is not a universal exclusive ownership law.

A **rule-value rewrite** consumes the matched whole value and produces its replacement. It does not assert behavioral equivalence or globally alias two names. The Rust runtime and JavaScript reference activate produced ground rule values locally, preserving lexical capture and read dependence. Exact whole-value matching and ordinary derivations can describe rules without decomposing their internal fields. `[[A,B]] ([B])` lowers as a nested empty-output rule match followed by a scope. Whole-rule replacement uses `[[A] B] [A] C`; see the [frontend contract](#syntax).

The Rust `rule::Rule::apply` implements generic literal multiset replacement, including exact whole nested rule values. It is separate from `runtime::Runtime`. That runtime and the JavaScript reference integrate joint inference, allocation, nested frames, dynamic activation of finite ground rule constructors, and positive evidence closure. The compiled code universe is fixed within each model instance; visible rule occurrences can change through ordinary events.

Source grammar rules and mathematical relations retain their conventional meanings. Earlier Photonic documents used relation for rule, world/partition for coherence, and join for decoherence. Preserve historical or scientific terms when describing their original subjects.

Photonic's coherence and decoherence are language-specific names, not a claim to simulate quantum physics. The [research report](research.html) records the physical comparisons and sources.

`Not`, `True`, and `False` are ordinary program-defined concepts. No label has built-in logical meaning, and there are no negative rule premises.
