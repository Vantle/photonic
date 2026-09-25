# Theorems

Photonic proves a claim by executing it. A theorem program states its claim in Photonic values, checks it, and concludes `Theorem` only when the claim holds. Prism checks the proof: the program must reach exactly `Theorem`, with every loaded rule.

Theorems are stated as generally as a finite check allows. The Boolean laws hold in every Boolean algebra; the relation and type theorems hold for every domain, every relation satisfying their hypotheses and every choice of component types; the group, lattice and ring laws hold in every group, lattice and ring; and the arithmetic laws hold at every width. Only the coloring theorems are specific numbers.

The last three layers prove the standard library itself. They ground its digit tables in counting, prove its chain cells for every item, and check its linked arithmetic one step at a time by running the library's own rules.

```sh
bazel test -c opt //theorem/...
```

## Notation

Every mathematical object is Photonic structure, so every part of a term is a value Photonic can match; no name hides structure.

| Object | Written | Example |
| --- | --- | --- |
| Element or constant | an atom | `X`, `One`, `Top` |
| Operation | a particle led by the operation, with ordered operands in `Left` and `Right` fields | `Times.([Left] X).([Right] Y)` |
| Commutative or unary operation | operands in `Of` fields | `Meet.([Of] X).([Of] Y)`, `Inverse.([Of] X)` |
| Proposition | a particle led by the relation | `Less.([Left] X).([Right] Y)` |
| Equation | two `Side` fields | `Equation.([Side] Times.([Left] One).([Right] Y)).([Side] Y)` |
| Sequence extended by an element | `Then` with the sequence and the element | `Then.([Left] X).([Right] A)` |
| Variable | a field keyed by what it names | `([P] True)`, `([Less.([Left] X).([Right] Y)] True)` |

Two `Of` fields form a multiset, so `Meet.([Of] X).([Of] Y)` and `Meet.([Of] Y).([Of] X)` are one value and commutativity needs no proof; likewise the two sides of an equation make symmetry free. An edge of a graph is the value `([Edge] A.B)`, one occurrence holding an unordered pair, so that a field keyed by it can replace it.

## Cases

[boolean/involution.wave](boolean/involution.wave) proves that negating twice returns the original value:

```
Case.(([P] True), ([P] False)),

[Claim] Function.Boolean.Not.Once.P,
[Return.Once] Function.Boolean.Not.Twice,
[Return.Twice] Function.Boolean.Equal.Verdict.P,

[Holds.(([P] True), ([P] False))] Theorem
```

The first line is the domain. Shared prefixes distribute, so it expands to one `Case` coherence per assignment. An assignment is a set of fields: `([P] True)` is a live rule that turns any `P` atom in its coherence into `True`, so `P` names the variable wherever the claim mentions it.

[case.particle](case.particle) gives every case its own scope:

```
[Case] (
    Claim,
    [Return.Verdict.True] Holds,
    [Return.Verdict.False] Counterexample
)
```

A true verdict returns `Holds` with the case's assignment to the root, and a false one returns `Counterexample`. The last line is the conclusion: its input expands to one `Holds` per assignment, so `Theorem` appears only once every case holds, and consuming the assignments leaves exactly `Theorem`.

### Nested quantifiers

A flat domain starts every case at once, so its program and its conclusion grow with the number of cases. A nested quantifier gives each variable a numbered scope that tries its values one at a time. From [coloring/sum.five.wave](coloring/sum.five.wave):

```
Every.1,

[Every.1] (
    Split.1,
    [Both.1] Theorem,
    [Refuted.2] Counterexample
),
[Split.1] Every.2.([One] True),
[Proved.2.([One] True)] Every.2.([One] False),
[Proved.2.([One] False)] Both.1
```

`Every.1` opens the scope for the first variable, which starts scope 2 with `One` set to True. When scope 2 reports `Proved.2`, the same report restarts it with `One` set to False, and the second report completes scope 1. Inner scopes report `Proved` instead of concluding `Theorem`; the innermost starts `Case` and waits for `Holds`. Only one case is in flight at a time.

Every rule that matches a field is written at the root. A value produced by a root rule captures the root's frame, and a whole-rule pattern only matches a value with the same capture, so a pattern written inside a scope never sees these fields. The scopes' own rules therefore match only the plain atoms `Both` and `Proved` with a level number. Root rules apply in every nested scope; the numbers name the reporting level, so a report from deeper inside never satisfies an outer rule. A failing case stops the search: `Counterexample` climbs through each scope as `Refuted` and reaches the root with the complete failing assignment.

## Claims

A claim reaches its verdict in one of four ways.

### Evaluated claims

An evaluated claim computes with the standard library's own operations, as `involution` does. Each rule waits for the answers it needs, calls the next operation and tags its answer; the last call is tagged `Verdict`. These claims are about the library's definitions.

### Covered claims

A covered claim is about relations nobody has defined. Its variables are propositions over generic elements, such as `Less.([Left] X).([Right] Y)`, the truth of x < y. Its cases are every truth assignment of those propositions. Each rule closes the cases that match a partial assignment, as `Vacuous` when the assignment falsifies an instance of a hypothesis, or `Concluded` when it satisfies the conclusion. [relation/asymmetry.wave](relation/asymmetry.wave) proves that every strict order is asymmetric:

```
[Claim] (Test, Keep),
[Test.([Less.([Left] X).([Right] X)] True)] Vacuous,
[Test.([Less.([Left] X).([Right] Y)] True).([Less.([Left] Y).([Right] X)] True).([Less.([Left] X).([Right] X)] False)] Vacuous,
[Test.([Less.([Left] X).([Right] Y)] False)] Concluded,
[Test.([Less.([Left] Y).([Right] X)] False)] Concluded
```

The first rule closes the cases where irreflexivity fails at x, and the second the cases where transitivity fails at x, y, x. The last two close the cases where x < y and y < x do not both hold. `Test` is matched and consumed while `Keep` holds the assignment for the report; [cover.particle](cover.particle) turns either closing into a true verdict.

A covered theorem holds for every domain and every relation satisfying its hypotheses. Choose any such structure and any elements for x and y: the propositions take one of the assignments, every hypothesis instance is true in it, so no `Vacuous` rule matches it and a `Concluded` rule must. Reviewing one means checking that each `Vacuous` rule falsifies an instance of a hypothesis and each `Concluded` rule implies the conclusion; the run checks that the rules close every case.

### Judged claims

A judged claim computes derived values from definitions before judging them, such as the verdict of comparing two sequences one position further. Definition rules read the assignment and write derived fields; from [sequence/converse.wave](sequence/converse.wave):

```
[Advance.Forward.([Compare.([Left] A).([Right] B)] Equal).([Compare.([Left] X).([Right] Y)] Less)] Advanced.([Compare.([Left] Then.([Left] X).([Right] A)).([Right] Then.([Left] Y).([Right] B))] Less)
```

If the next elements compare Equal and the sequences so far compare Less, the extended sequences compare Less. A judge table over every combination of the derived fields answers `Upheld` or `Overturned`. The judge consumes every derived field, so none leaks into the next case's report. [judge.particle](judge.particle) holds a case when it is upheld, or when it is overturned but a `Vacuous` rule closes it.

### Derived claims

A derived claim is an equational proof. Each fact is a coherence holding one equation: an instance of an axiom, a premise of the theorem, or an instance of an earlier theorem. Each rule is congruence, which puts both sides of an equation into the same context, or transitivity. From [group/inverse.wave](group/inverse.wave), which proves that xy = 1 implies y = x⁻¹ in every group:

```
Equation.([Side] Times.([Left] X).([Right] Y)).([Side] One),
Equation.([Side] Times.([Left] One).([Right] Y)).([Side] Y),
Equation.([Side] Times.([Left] Inverse.([Of] X)).([Right] X)).([Side] One),
...

[Equation.([Side] Times.([Left] X).([Right] Y)).([Side] One)] Equation.([Side] Times.([Left] Inverse.([Of] X)).([Right] Times.([Left] X).([Right] Y))).([Side] Times.([Left] Inverse.([Of] X)).([Right] One)),
[Equation.([Side] Times.([Left] One).([Right] Y)), Equation.([Side] Times.([Left] One).([Right] Y))] Equation,
...

[Equation.([Side] Y).([Side] Inverse.([Of] X))] Theorem
```

The first rule multiplies both sides of xy = 1 on the left by x⁻¹. The second is transitivity through 1y: it joins two equations that share that side, consumes the shared side from both, and the remainder law leaves the two outer sides in one equation. A transitivity rule therefore names only its middle term. The last rule states the conclusion.

Every fact is true in every structure of the kind, for every choice of the generic elements satisfying the premises, and every rule turns true equations into a true one, so the conclusion holds in every such structure. Rules consume their premises, so reaching exactly `Theorem` also shows every listed fact was used; a fact needed twice is listed twice. A proof may need its rules in a particular order, so derived claims are checked by Prism's full exploration: reaching `Theorem` means some order of the rules derives it. Reviewing one means checking that each fact is an axiom instance, a premise or a cited theorem, and that each congruence rule applies one context to both sides of its premise.

### Induction

Where a statement is about sequences of every length or numbers of every width, the claim is an induction step. Its cases are every state the computation carries between positions and every next element, and it checks that an invariant over the state holds after the position whenever it held before. The invariant holds for the empty sequence, so induction on the length, argued here rather than executed, gives every length.

### Rules about rules

A claim can take another program's rules as its subject. The layer 9 claims start a library engine in a state its own rules reach between two steps, let those rules take one step, and meet what they did with what the digit tables require. Rules stand in for the data the step never reaches: a feed answers the chain protocol with the case's next digit, followed by a remnant that answers nothing but `Forget`. A case can assign a whole piece of engine state, so `([M] Mode.([Operation] Add))` turns `Column.M` into the column engine's add mode, itself a rule value. The claim reads the engine's output back through the chain's own methods, rule values that the chain handle carries. Layer 8 proves those methods correct.

## What a proof establishes

Reaching `Theorem` means every case closed with a true verdict, or the equations derived the conclusion, because:

- **Cases are isolated.** A rule combines coherences within one frame, and each case runs in its own scope, so no rule can pair values from different cases.
- **Evaluations are functions.** Every library operation and definition table a claim uses has one rule per combination of operands, and every call coherence holds exactly its operands, so each call has exactly one answer.
- **The domain is complete.** A flat domain is written twice, as the initial cases and as the conclusion's premises. A nested quantifier writes each variable's values once as the chain that tries them and once as the report that completes it.
- **Nothing is left over.** The target is exact, so a stray value in any case, or an unused fact, prevents the proof.

The runtime and Prism are trusted to execute the rules. In layers 1 to 6 the library's rules are the definitions an evaluated theorem is about; layers 7 to 9 prove those definitions in turn, down to the successor of each trit. Prism itself checks one concrete reachability claim; the arguments above turn reaching `Theorem` into a statement about every case, and the covering, derivation and induction arguments turn that into the general statement. Every direct execution of a case-based claim reaches `Theorem`, so following one path suffices there.

A layer 9 claim also relies on two facts about the engine it runs, both visible in the engine's rules. Its starting state is one the engine reaches between steps, and a feed behaves as a chain for one step. The engines touch an operand only by sending `Read`, `Peek` and `Forget`, and they test only for the empty chain `Zero`, which `Feed.End` answers as.

A false claim reaches `Counterexample` instead, and a refutation test checks that configuration exactly. [boolean/negation.wave](boolean/negation.wave) claims ¬(p ∧ q) = ¬p ∧ ¬q and ends with counterexamples at (True, False) and (False, True) beside the two cases that hold. A nested claim stops at its first counterexample, so its refutation names one assignment.

## Writing a theorem

- Write every term, proposition and equation in the notation above; never name a structured object with one atom. Only elements and constants are atoms.
- Write each variable's domain once in the cases and once in the conclusion: `(([X] Kill), ([X] Propagate), ([X] Generate))`. Use a nested quantifier once a flat domain would need more than a few dozen cases.
- A variable that is substituted, rather than matched by a covering rule, must be one occurrence: an atom such as `P` or a value such as `([Edge] A.B)`. A field whose input is a particle fires on atoms meant for other variables.
- Tag every call with words the loaded libraries do not use. A pattern matches any coherence containing its atoms, so no tag may contain another. Theorems stating two laws use `Lower` and `Upper` as namespaces.
- Keep each value that still has to meet another in its own coherence. Every output of a rule receives its unmatched remainder, so splitting a coherence that holds a computed value copies that value.
- Pass role operands through an adapter. [carry/role.particle](carry/role.particle) packs a status after `Former` into `([Left] …)` and after `Latter` into `([Right] …)`, marking each `Packed`; a joint rule over two `Packed` coherences then makes the call. [ternary/role.particle](ternary/role.particle) does the same for trits and borrows.
- Consume every value. A result that is not needed still has to be matched, or discarded by a rule with no output such as `[High.Joined],`; otherwise it remains in the final configuration and the proof fails.
- Guard every rule that observes a running engine with a value that exists only after the step, such as a read blocked on a feed's remnant or an output that now holds a cell. An observer that could fire earlier takes the engine's own inputs and stalls it.
- Declare the theorem with `theorem` from [defs.bzl](defs.bzl), which builds the program and its `.proof` test; `path = False` checks a derivation by full exploration. `refutation` builds a `.refutation` test that checks the configuration a false claim ends in. Raise `cells` as the claim grows and `states` as the path grows: a direct path retains every configuration it visits.

## Order

Layers 1 to 6 climb from propositional logic to algebraic structures. Each is more abstract than the definitions below it, or lifts laws proved below it to every size. Layers 7 to 9 turn to the standard library itself. They ground its digit tables in counting and build up to its linked arithmetic.

| Layer | Packages | Theorems |
| --- | --- | ---: |
| 1. Propositional logic | [boolean](boolean/) | 12 |
| 2. Relations | [relation](relation/) | 5 |
| 3. Constructed types | [componentwise](componentwise/), [lexicographic](lexicographic/), [sum](sum/), [sequence](sequence/), [ternary](ternary/) | 14 |
| 4. Arithmetic at every width | [carry](carry/), [induction](induction/) | 6 |
| 5. Exact combinatorics | [coloring](coloring/) | 8 |
| 6. Algebraic structures | [monoid](monoid/), [group](group/), [lattice](lattice/), [ring](ring/), [ternary](ternary/) | 21 |
| 7. Counting | [counting](counting/) | 6 |
| 8. Linked storage | [chain](chain/) | 3 |
| 9. Linked arithmetic | [natural](natural/) | 9 |

### 1. Propositional logic

The library's Booleans form a Boolean algebra: identity, complement and distributivity, with commutativity free, are Huntington's axioms. Every Boolean algebra embeds in a power of the two-element one, so an identity that holds for `Boolean.And`, `Boolean.Or` and `Boolean.Not` in every case holds in every Boolean algebra.

| Theorem | Claim |
| --- | --- |
| [involution](boolean/involution.wave) | ¬¬p = p |
| [complement](boolean/complement.wave) | p ∧ ¬p = False, p ∨ ¬p = True |
| [idempotence](boolean/idempotence.wave) | p ∧ p = p, p ∨ p = p |
| [identity](boolean/identity.wave) | p ∧ True = p, p ∨ False = p |
| [domination](boolean/domination.wave) | p ∧ False = False, p ∨ True = True |
| [absorption](boolean/absorption.wave) | p ∧ (p ∨ q) = p, p ∨ (p ∧ q) = p |
| [duality](boolean/duality.wave) | ¬(p ∧ q) = ¬p ∨ ¬q, ¬(p ∨ q) = ¬p ∧ ¬q |
| [associativity](boolean/associativity.wave) | (p ∧ q) ∧ r = p ∧ (q ∧ r), (p ∨ q) ∨ r = p ∨ (q ∨ r) |
| [distributivity](boolean/distributivity.wave) | p ∧ (q ∨ r) = (p ∧ q) ∨ (p ∧ r), p ∨ (q ∧ r) = (p ∨ q) ∧ (p ∨ r) |
| [equivalence](boolean/equivalence.wave) | `Boolean.Equal` is reflexive and transitive |
| [peirce](boolean/peirce.wave) | ((p → q) → p) → p, with p → q as ¬p ∨ q |
| [negation](boolean/negation.wave) | refuted: ¬(p ∧ q) = ¬p ∧ ¬q fails at (True, False) and (False, True) |

### 2. Relations

Covered theorems about any relation on any domain. Reflexivity and symmetry of the derived relations follow from their definitions, so each theorem proves the property that needs the hypotheses.

| Theorem | Claim |
| --- | --- |
| [asymmetry](relation/asymmetry.wave) | a strict order is asymmetric |
| [kernel](relation/kernel.wave) | x ≤ y ∧ y ≤ x is transitive for every preorder, so it is an equivalence |
| [strict](relation/strict.wave) | x ≤ y ∧ ¬(y ≤ x) is transitive for every preorder, so it is a strict order |
| [trichotomy](relation/trichotomy.wave) | in a total preorder, any two elements are strictly ordered one way or equivalent |
| [partition](relation/partition.wave) | two classes of an equivalence that share an element coincide |

### 3. Constructed types

Orders lift through type constructors. Each theorem assumes only the order axioms of its component types, so it holds for every choice of components, including constructed ones. A pair of A × B is written (X, U), (Y, V) or (Z, W); elements of A + B are p, q and r, and `([Side.P] Left)` says p lies in A.

| Theorem | Claim |
| --- | --- |
| [componentwise.reflexive](componentwise/reflexive.wave), [componentwise.antisymmetric](componentwise/antisymmetric.wave), [componentwise.transitive](componentwise/transitive.wave) | the componentwise order on A × B is a partial order when both components are |
| [lexicographic.irreflexive](lexicographic/irreflexive.wave), [lexicographic.transitive](lexicographic/transitive.wave), [lexicographic.trichotomy](lexicographic/trichotomy.wave) | the lexicographic order on A × B is a strict total order when both components are |
| [sum.irreflexive](sum/irreflexive.wave), [sum.transitive](sum/transitive.wave), [sum.trichotomy](sum/trichotomy.wave) | ordering all of A before all of B makes A + B a strict total order when both are |
| [sequence.transitive](sequence/transitive.wave), [sequence.converse](sequence/converse.wave), [sequence.equality](sequence/equality.wave) | comparing equal-length sequences least significant first, where the latest difference decides, is a strict total order whose equality is positionwise, at every length |
| [ternary.order](ternary/order.wave), [ternary.converse](ternary/converse.wave) | `Ternary.Compare` orders the trits: any three compare consistently, swapping the operands flips the answer, and Equal means `Ternary.Equal` |

The sequence theorems are induction steps whose state is the verdict so far, one of Less, Equal and Greater. The element type enters only through abstract comparisons `Compare.([Left] A).([Right] B)` of the elements at the next position, constrained by its order: any three comparisons are consistent, and swapping two operands flips theirs. Transitivity keeps the verdicts among three sequences consistent over all 729 combinations of verdicts and comparisons.

Induction over type expressions, argued rather than executed, combines these: every type built from trits with lexicographic products, sums and equal-length sequences is strictly totally ordered, and componentwise products of such types are partially ordered. Unfolding A* as 1 + A × A* and applying the sum and lexicographic theorems at each length orders sequences of different lengths too, with a proper prefix first.

In particular, `Natural.Compare`'s scheme orders the naturals. It compares digits least significant first, the most significant difference decides, and a finished operand reads as zero, which pads both numerals to one width; with the trits ordered by `Ternary.Compare`, the sequence theorems apply. Layer 9 proves that the linked implementation follows this scheme.

### 4. Arithmetic at every width

A block of digits kills, propagates or generates a carry. `Carry.Combine` joins a lower block, its `Left` operand, with a higher one, its `Right` operand; `Carry.Evaluate` applies a block to an incoming carry. The carry theorems show the statuses form a monoid acting on carries:

| Theorem | Claim |
| --- | --- |
| [identity](carry/identity.wave) | Propagate ∘ x = x = x ∘ Propagate |
| [associativity](carry/associativity.wave) | (x ∘ y) ∘ z = x ∘ (y ∘ z) |
| [action](carry/action.wave) | Evaluate(l ∘ h, c) = Evaluate(h, Evaluate(l, c)) |

The induction theorems carry digit laws to numbers of every width. Each names an invariant over the carries passed between columns; a number is padded with zero columns until every carry is spent.

| Theorem | Claim at every width | Invariant | Cases |
| --- | --- | --- | ---: |
| [lookahead](induction/lookahead.wave) | combining blocks with `Carry.Combine` and applying the result gives the ripple carry | the combined prefix takes the entry carry to the ripple carry | 36 |
| [subtraction](induction/subtraction.wave) | subtracting b from a + b with `Ternary.Subtract` returns a | the adder's carry equals the subtractor's borrow | 18 |
| [associativity](induction/associativity.wave) | (a + b) + c = a + (b + c) with `Ternary.Sum` | both groupings hold the same total pending carry | 432 |

Lookahead also checks its base in every case: Propagate, the combination of no blocks, leaves the entry carry unchanged. The other two start with every carry and borrow at zero, where their invariants hold by definition. Associativity and lookahead are the conditions under which any parallel prefix network computes the same carries as a ripple adder.

Their proofs need two adapters: [ternary/role.particle](ternary/role.particle) packs trits into the `Left`, `Right` and `Borrow` operands of `Ternary.Subtract`, and [ternary/unpack.particle](ternary/unpack.particle) splits every `([Digit] d).([Carry] c)` or `([Digit] d).([Borrow] b)` answer into a `Low` coherence holding the digit and a `High` coherence holding the carry or borrow.

### 5. Exact combinatorics

Schur's, van der Waerden's and Ramsey's theorems need arguments beyond a finite check, so this layer proves exact values instead: each claim is proved at one size and refuted at the size below it. The nested quantifiers find the refuting colorings themselves, each the first counterexample in the order the scopes try values, True before False.

| Theorem | Claim | Cases |
| --- | --- | ---: |
| [pigeonhole.four](coloring/pigeonhole.four.wave) | any four trits contain two equal ones | 81 |
| [pigeonhole.three](coloring/pigeonhole.three.wave) | refuted by 0, 1, 2 | |
| [sum.five](coloring/sum.five.wave) | every 2-coloring of 1..5 colors some x, y and x + y alike | 32 |
| [sum.four](coloring/sum.four.wave) | refuted by coloring 1 and 4 True, 2 and 3 False | |
| [progression.nine](coloring/progression.nine.wave) | every 2-coloring of 1..9 colors some 3-term arithmetic progression alike | 512 |
| [progression.eight](coloring/progression.eight.wave) | refuted by True, True, False, False, True, True, False, False | |
| [triangle.six](coloring/triangle.six.wave) | every 2-coloring of the edges of K6 has a triangle colored alike | 326 closed branches |
| [triangle.five](coloring/triangle.five.wave) | refuted by a pentagon of True edges and a pentagram of False ones | |

Together they establish the Schur number S(2) = 4, the van der Waerden number W(2,3) = 9 and the Ramsey number R(3,3) = 6. A triple is uniform when its three colors agree, which the claims define with four rules over the unordered triple; a verdict is the disjunction of the triples' uniformity, folded with `Boolean.Or`.

The triangle theorems close branches early. K6 has 32,768 colorings, but a partial coloring that already contains a uniform triangle settles every coloring extending it. Each edge's scope first tests the triangles its predecessor completed, in an order that visits edges by their larger vertex; a uniform one reports `Proved` without splitting further. The proof is then a tree of 651 scopes whose 326 leaves each name a uniform triangle, and it runs 9,573 events. W(2,3) enumerates all 512 colorings in 52,219 events.

### 6. Algebraic structures

Derived theorems about every monoid, group, lattice and ring, and the library's structures as instances. A theorem cited by a later one appears there as a fact or rule instantiating it.

| Theorem | Claim | Cites |
| --- | --- | --- |
| [monoid.identity](monoid/identity.wave) | an element e with ex = x for every x equals 1 | |
| [group.inverse](group/inverse.wave) | xy = 1 implies y = x⁻¹ | |
| [group.cancellation](group/cancellation.wave) | xy = xz implies y = z | |
| [group.involution](group/involution.wave) | (x⁻¹)⁻¹ = x | inverse |
| [group.unit](group/unit.wave) | 1⁻¹ = 1 | inverse |
| [group.reversal](group/reversal.wave) | (xy)⁻¹ = y⁻¹x⁻¹ | inverse |
| [lattice.idempotence](lattice/idempotence.wave) | x ∧ x = x, x ∨ x = x | |
| [lattice.order](lattice/order.wave), [lattice.converse](lattice/converse.wave) | x ∧ y = x exactly when x ∨ y = y, so both define one order x ≤ y | |
| [lattice.antisymmetry](lattice/antisymmetry.wave), [lattice.transitivity](lattice/transitivity.wave) | that order is antisymmetric and transitive; idempotence makes it reflexive | |
| [lattice.distributivity](lattice/distributivity.wave) | if meet distributes over join, join distributes over meet | |
| [lattice.modularity](lattice/modularity.wave) | a distributive lattice is modular: x ≤ z implies (x ∨ y) ∧ z = x ∨ (y ∧ z) | |
| [lattice.complement](lattice/complement.wave) | in a bounded distributive lattice, a complement is unique | |
| [ring.annihilation.right](ring/annihilation.right.wave), [ring.annihilation.left](ring/annihilation.left.wave) | x · 0 = 0 and 0 · x = 0 | |
| [ring.negation](ring/negation.wave) | (−x) · y = −(x · y) = x · (−y) | annihilation, inverse |
| [ring.sign](ring/sign.wave) | (−x) · (−y) = x · y | negation, involution |
| [ring.unit](ring/unit.wave) | (−1) · x = −x in a ring with identity | negation |
| [ternary.group](ternary/group.wave) | the trits under the digit of `Ternary.Add` form the group of order 3, with inverses from `Ternary.Subtract` | |
| [ternary.field](ternary/field.wave) | with the digit of `Ternary.Multiply` they form the field of order 3: multiplication is associative and distributes over addition, 1 is its identity, and every nonzero trit is its own inverse | |

A ring's addition is a commutative group, so the ring theorems cite the group theorems in additive notation: `inverse` reads a + b = 0 implies b = −a, and `involution` reads −(−a) = a.

The instances connect these to the library. `ternary.group` and `ternary.field` make every group and ring theorem hold for the trits. The carry theorems of layer 4 make the statuses a monoid under `Carry.Combine`, so `monoid.identity` shows Propagate is its only identity. The Boolean laws of layer 1 include absorption, associativity, idempotence and both distributive laws for `Boolean.And` and `Boolean.Or`, so the Booleans are a distributive lattice and every lattice theorem holds for them.

### 7. Counting

The digit tables are definitions: nothing in layers 1 to 6 says that `Ternary.Sum` adds. This layer grounds them in counting. It trusts only two things: the successor of each trit, `Ternary.Successor`, and the meaning of a numeral as that many successor steps. [counting/count.particle](counting/count.particle) writes that meaning once:

```
[Walk.2] (Stride, Stride),
[Need.2, Receipt, Receipt] Satisfied,
[Stride, Place] Function.Ternary.Successor.Moving,
[Low.Moving] Place,
[High.0.Moving] Receipt,
[High.1.Moving, Wraps] Function.Ternary.Successor.Wrapping,
[Low.Wrapping] Wraps,
[High.0.Wrapping] Receipt
```

A walk of n strides moves `Place` n successors forward and counts in `Wraps` how often it passes 2. Each stride leaves a `Receipt`, and `Need.n` is satisfied by n of them. Photonic cannot observe that no stride remains, so the receipts let a claim learn positively that every stride has landed. [counting/sum.wave](counting/sum.wave) checks all 27 sums:

```
[Claim] (Place.A, Wraps.0, Walk.B, Walk.C, Need.B, Need.C, Function.Ternary.Sum.Expected.A.B.C),
[Satisfied, Satisfied, Place.1, Wraps.1, Low.1.Expected, High.1.Expected] Return.Verdict.True
```

The second rule is one of nine. Sum's digit must be where b + c strides from a land, and its carry must be how often they wrapped.

| Theorem | Claim |
| --- | --- |
| [cycle](counting/cycle.wave) | three strides from any trit return to it after exactly one wrap: the successor is one cycle through the trits |
| [add](counting/add.wave) | `Ternary.Add` of a and b is where b strides from a land, carrying the wraps |
| [sum](counting/sum.wave) | `Ternary.Sum` of a, b and c is b + c strides from a |
| [subtract](counting/subtract.wave) | `Ternary.Subtract` inverts counting: b + w strides from the difference of a and b with borrow w land on a, wrapping exactly when it borrows |
| [multiply](counting/multiply.wave) | `Ternary.Multiply` of a and b is b rounds of a strides from 0 |
| [compare](counting/compare.wave) | `Ternary.Compare` is the sign of the difference: Less exactly when a − b borrows, Equal when it leaves 0 |

### 8. Linked storage

A chain handle is a coherence that carries its own methods as rule values. [cell.particle](../library/chain/cell.particle) gives each cell `Read`, `Peek` and `Forget` methods. They are sealed by the capture of the rules that built them, and a handle is destroyed by matching its methods whole. The storage theorems hold for every item and every chain below it. The cell's rules mention neither `Item` nor `Below`, so the run with these atoms is the run for any others. The item's alphabet must declare `Drop`, as the digits do, and the chain below must answer `Forget`, as every chain does. From [chain/read.wave](chain/read.wave):

```
Push.Item.Below,

[Drop.Item] Forget,
[Forget.Below] Clean,
[Built] Read,
[Yield.Item.Below] Theorem
```

| Theorem | Claim |
| --- | --- |
| [read](chain/read.wave) | reading a pushed cell returns exactly its item and the chain below, and destroys the handle's methods |
| [peek](chain/peek.wave) | peeking returns the same and keeps the cell, so another reference still reads it |
| [forget](chain/forget.wave) | forgetting one reference to a cell leaves the cell for another |

### 9. Linked arithmetic

The linked engines read numbers of any width, so their theorems are induction steps like those of layer 4. The difference is what they run: the library's own rules, not a scheme. Each claim starts an engine in a state it holds between two steps, and hands it feeds instead of chains. From [natural/feed.particle](natural/feed.particle):

```
[Read.Feed.1] Yield.([Digit] 1).Remnant,
[Read.Feed.End] Yield.End.Zero,
[Forget.Remnant] Clean,
[Forget.Prior] Clean
```

A feed answers `Read` or `Peek` with the case's next digit, followed by a `Remnant` that answers nothing but `Forget`. `Prior` stands for the output built so far. A step reads at most one digit from each operand, so a feed stands for every chain with that next digit. `Feed.End` answers as the empty chain `Zero`, which it stands for. The engine's rules take one step and block on the remnants, and the claim reads the output back through the real chain. From [natural/addition.wave](natural/addition.wave):

```
[Claim] (Column.Step, Column.Mode.([Operation] Add), Column.Carry.K, Column.Left.Feed.A, Column.Right.Feed.B, Column.Output.Prior, Worth.A, Worth.B, Function.Ternary.Sum.Expected.K),
[Column.Output.Head, Read.Column.Reader.([Side] Left).Remnant] Read.Probe.Head,
[Yield.Probe.Prior, Column.Carry, Column.Mode.([Operation] Add), Read.Column.Reader.([Side] Right).Remnant] Engine,
[Engine.([Digit] 1).1, Low.1.Expected, High.1.Expected] Return.Verdict.True
```

The probe fires only once the output holds a new cell and the engine has begun the next column. The digit under the probe must be the digit of `Ternary.Sum`, with `End` read as 0, and the engine's new carry must be Sum's carry. The tail `Prior` shows that the engine pushed exactly one digit onto whatever the output already held. An operand that has ended reads `End` again at the next column instead of blocking, and two more observers cover those states.

| Theorem | Step | Cases |
| --- | --- | ---: |
| [opening](natural/opening.wave) | the first column of `Natural.Add`, run from the call itself, pushes the digit of `Ternary.Sum` of the two digits and no carry onto an empty output and passes on Sum's carry; 0 + 0 answers zero | 16 |
| [addition](natural/addition.wave) | a column of `Natural.Add` pushes the digit of `Ternary.Sum` of the two digits read and the carry, and passes on Sum's carry | 30 |
| [subtraction](natural/subtraction.wave) | a column in the borrow mode behind `Natural.Subtract` and `Natural.Difference` pushes the digit of `Ternary.Subtract` and passes on its borrow | 30 |
| [ending](natural/ending.wave) | once both operands have ended, a final carry becomes the leading digit and a final borrow reports a negative difference | 4 |
| [comparison](natural/comparison.wave) | a position of `Natural.Compare` carries on the verdict of the layer 3 scheme, or answers it once one operand has ended and the other shows a nonzero digit, or both have ended | 48 |
| [trim](natural/trim.wave) | `Natural.Trim` skips a leading zero, answers zero once only zeros remain, and otherwise keeps the leading digit and reverses the chain | 4 |
| [reversal](natural/reversal.wave) | each step of `Chain.Reverse` moves one digit onto its result, which it returns at the end | 4 |
| [successor](natural/successor.wave) | `Natural.Successor` turns a trailing 2 into a pending 0, and otherwise writes the digit's successor | 4 |
| [restoration](natural/restoration.wave) | each pending 0 then returns beneath the new digit, and the number is returned | 2 |

With layer 7 these give the library's linked arithmetic at every width, by induction on the steps. `opening` shows that `Natural.Add` starts with carry 0 and an empty output. After each column, `addition` and `sum` keep the output equal to the low digits of a + b, most significant on top, with the carry owed to the next column. `ending` pushes the last carry, and `trim` and `reversal` return the numeral least significant first with no leading zeros. The same argument with `subtraction` and `subtract` gives a − b, and `ending` reports a negative difference exactly when b > a. `comparison` shows that `Natural.Compare` follows the scheme that layer 3 proved orders the naturals, and `compare` orders its digits by the sign of their difference. Its early answers are final. Once the left operand has ended, every later position compares 0 with a digit, which can turn Equal into Less but never undoes Less, and symmetrically for Greater. `successor`, `restoration` and `cycle` give n + 1.

`comparison` pins down the engine's exact stopping rule. Consider an engine that answers Less one position earlier, where the left operand has ended and the right shows 0 after an earlier Less. It would still be correct, but it fails this theorem.
