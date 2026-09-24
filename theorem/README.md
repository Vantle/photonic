# Theorems

Photonic proves a claim by executing it. A theorem program states its claim in Photonic values, checks it, and concludes `Theorem` only when the claim holds. Prism checks the proof: the program must reach exactly `Theorem`, with every loaded rule.

Theorems are stated as generally as a finite check allows. The Boolean laws hold in every Boolean algebra; the relation and type theorems hold for every domain, every relation satisfying their hypotheses and every choice of component types; and the arithmetic laws hold at every width. Only the coloring theorems are specific numbers.

```sh
bazel test -c opt //theorem/...
```

## Notation

Every mathematical object is Photonic structure, so every part of a term is a value Photonic can match; no name hides structure.

| Object | Written | Example |
| --- | --- | --- |
| Element or constant | an atom | `X`, `True`, `Generate` |
| Proposition | a particle led by the relation | `Less.([Left] X).([Right] Y)` |
| Sequence extended by an element | `Then` with the sequence and the element | `Then.([Left] X).([Right] A)` |
| Variable | a field keyed by what it names | `([P] True)`, `([Less.([Left] X).([Right] Y)] True)` |

An edge of a graph is the value `([Edge] A.B)`, one occurrence holding an unordered pair, so that a field keyed by it can replace it.

## Cases

[boolean/involution.wave](boolean/involution.wave) proves that negating twice returns the original value:

```
Case.(([P] True), ([P] False))

[Claim] Function.Boolean.Not.Once.P
[Return.Once] Function.Boolean.Not.Twice
[Return.Twice] Function.Boolean.Equal.Verdict.P

[Holds.(([P] True), ([P] False))] Theorem
```

The first line is the domain. Shared prefixes distribute, so it expands to one `Case` coherence per assignment. An assignment is a set of fields: `([P] True)` is a live rule that turns any `P` atom in its coherence into `True`, so `P` names the variable wherever the claim mentions it.

[case.particle](case.particle) gives every case its own scope:

```
[Case] (
    Claim
    [Return.Verdict.True] Holds
    [Return.Verdict.False] Counterexample
)
```

A true verdict returns `Holds` with the case's assignment to the root, and a false one returns `Counterexample`. The last line is the conclusion: its input expands to one `Holds` per assignment, so `Theorem` appears only once every case holds, and consuming the assignments leaves exactly `Theorem`.

### Nested quantifiers

A flat domain starts every case at once, so its program and its conclusion grow with the number of cases. A nested quantifier gives each variable a numbered scope that tries its values one at a time. From [coloring/sum.five.wave](coloring/sum.five.wave):

```
Every.1

[Every.1] (
    Split.1
    [Both.1] Theorem
    [Refuted.2] Counterexample
)
[Split.1] Every.2.([One] True)
[Proved.2.([One] True)] Every.2.([One] False)
[Proved.2.([One] False)] Both.1
```

`Every.1` opens the scope for the first variable, which starts scope 2 with `One` set to True. When scope 2 reports `Proved.2`, the same report restarts it with `One` set to False, and the second report completes scope 1. Inner scopes report `Proved` instead of concluding `Theorem`; the innermost starts `Case` and waits for `Holds`. Only one case is in flight at a time.

Every rule that matches a field is written at the root. A value produced by a root rule captures the root's frame, and a whole-rule pattern only matches a value with the same capture, so a pattern written inside a scope never sees these fields. The scopes' own rules therefore match only the plain atoms `Both` and `Proved` with a level number. Root rules apply in every nested scope; the numbers name the reporting level, so a report from deeper inside never satisfies an outer rule. A failing case stops the search: `Counterexample` climbs through each scope as `Refuted` and reaches the root with the complete failing assignment.

## Claims

A claim reaches its verdict in one of three ways.

### Evaluated claims

An evaluated claim computes with the standard library's own operations, as `involution` does. Each rule waits for the answers it needs, calls the next operation and tags its answer; the last call is tagged `Verdict`. These claims are about the library's definitions.

### Covered claims

A covered claim is about relations nobody has defined. Its variables are propositions over generic elements, such as `Less.([Left] X).([Right] Y)`, the truth of x < y. Its cases are every truth assignment of those propositions. Each rule closes the cases that match a partial assignment, as `Vacuous` when the assignment falsifies an instance of a hypothesis, or `Concluded` when it satisfies the conclusion. [relation/asymmetry.wave](relation/asymmetry.wave) proves that every strict order is asymmetric:

```
[Claim] (Test) (Keep)
[Test.([Less.([Left] X).([Right] X)] True)] Vacuous
[Test.([Less.([Left] X).([Right] Y)] True).([Less.([Left] Y).([Right] X)] True).([Less.([Left] X).([Right] X)] False)] Vacuous
[Test.([Less.([Left] X).([Right] Y)] False)] Concluded
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

### Induction

Where a statement is about sequences of every length or numbers of every width, the claim is an induction step. Its cases are every state the computation carries between positions and every next element, and it checks that an invariant over the state holds after the position whenever it held before. The invariant holds for the empty sequence, so induction on the length, argued here rather than executed, gives every length.

## What a proof establishes

Reaching `Theorem` means every case closed with a true verdict, because:

- **Cases are isolated.** A rule combines coherences within one frame, and each case runs in its own scope, so no rule can pair values from different cases.
- **Evaluations are functions.** Every library operation and definition table a claim uses has one rule per combination of operands, and every call coherence holds exactly its operands, so each call has exactly one answer.
- **The domain is complete.** A flat domain is written twice, as the initial cases and as the conclusion's premises. A nested quantifier writes each variable's values once as the chain that tries them and once as the report that completes it.
- **Nothing is left over.** The target is exact, so a stray value in any case prevents the proof.

The runtime and Prism are trusted to execute the rules, and the library's rules are the definitions an evaluated theorem is about. Prism itself checks one concrete reachability claim; the arguments above turn reaching `Theorem` into a statement about every case, and the covering and induction arguments turn that into the general statement. Every direct execution of a case-based claim reaches `Theorem`, so following one path suffices there.

A false claim reaches `Counterexample` instead, and a refutation test checks that configuration exactly. [boolean/negation.wave](boolean/negation.wave) claims ¬(p ∧ q) = ¬p ∧ ¬q and ends with counterexamples at (True, False) and (False, True) beside the two cases that hold. A nested claim stops at its first counterexample, so its refutation names one assignment.

## Writing a theorem

- Write every term and proposition in the notation above; never name a structured object with one atom. Only elements and constants are atoms.
- Write each variable's domain once in the cases and once in the conclusion: `(([X] Kill), ([X] Propagate), ([X] Generate))`. Use a nested quantifier once a flat domain would need more than a few dozen cases.
- A variable that is substituted, rather than matched by a covering rule, must be one occurrence: an atom such as `P` or a value such as `([Edge] A.B)`. A field whose input is a particle fires on atoms meant for other variables.
- Tag every call with words the loaded libraries do not use. A pattern matches any coherence containing its atoms, so no tag may contain another. Theorems stating two laws use `Lower` and `Upper` as namespaces.
- Keep each value that still has to meet another in its own coherence. Every output of a rule receives its unmatched remainder, so splitting a coherence that holds a computed value copies that value.
- Pass role operands through an adapter. [carry/role.particle](carry/role.particle) packs a status after `Former` into `([Left] …)` and after `Latter` into `([Right] …)`, marking each `Packed`; a joint rule over two `Packed` coherences then makes the call. [ternary/role.particle](ternary/role.particle) does the same for trits and borrows.
- Consume every value. A result that is not needed still has to be matched, or discarded by a rule with no output such as `[High.Joined],`; otherwise it remains in the final configuration and the proof fails.
- Declare the theorem with `theorem` from [defs.bzl](defs.bzl), which builds the program and its `.proof` test. `refutation` builds a `.refutation` test that checks the configuration a false claim ends in. Raise `cells` as the claim grows and `states` as the path grows: a direct path retains every configuration it visits.

## Order

Each layer is more abstract than the definitions below it, or lifts laws proved below it to every size.

| Layer | Packages | Theorems |
| --- | --- | ---: |
| 1. Propositional logic | [boolean](boolean/) | 12 |
| 2. Relations | [relation](relation/) | 5 |
| 3. Constructed types | [componentwise](componentwise/), [lexicographic](lexicographic/), [sum](sum/), [sequence](sequence/), [ternary](ternary/) | 14 |
| 4. Arithmetic at every width | [carry](carry/), [induction](induction/) | 6 |
| 5. Exact combinatorics | [coloring](coloring/) | 8 |

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

In particular, `Natural.Compare`'s scheme orders the naturals. It compares digits least significant first, the most significant difference decides, and a finished operand reads as zero, which pads both numerals to one width; with the trits ordered by `Ternary.Compare`, the sequence theorems apply. This concerns the scheme, not yet the linked implementation.

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
