# Arithmetic

[Webbook](../index.html) · [Repository guide](../README.md)

- [Natural numbers](#natural)
- [Addition laws](#addition)
- [Multiplication by repeated addition](#repetition)
- [Multiplication from two runtime numerals](#product)
- [Ternary arithmetic](#word)
- [Native expressions](expression.md)
- [Ternary streams](#stream)
- [Arithmetic and parallel execution](#algorithm)
- [Choosing a compact representation](#representation)
- [Finite decimals](#decimal)

<a id="natural"></a>

## Natural numbers

This is a mathematical specification of the first Photonic number representation, with written arguments and executable finite checks. The ambient reasoning assumes finite constructions, equality, and induction over those constructions. It is not yet a foundational calculus or a universally quantified proof checked inside Photonic.

<a id="natural-carrier"></a>

### Carrier

A numeral occupies one root coherence. Its particle contains only occurrences of the ordinary concept `Unit`. All occurrences have distinct introductions within that coherence. There are no captured values, local frames, or unrelated concepts in the representation.

Start with the empty particle. Repeatedly add one fresh Unit, finitely many times. Admit exactly the results of these constructions. Identify presentations that differ only in occurrence names or order. Do not identify different multiplicities.

This is a least-closure definition: it admits no extra elements beyond those generated from the empty particle. Merely declaring an arbitrary zero and a successor operation would not exclude additional elements unrelated to zero.

| Mathematical notation | Photonic representation |
| --- | --- |
| 0 | `()` |
| S(0) | `Unit` |
| S(S(0)) | `Unit.Unit` |
| S(S(S(0))) | `Unit.Unit.Unit` |

The notation S and the decimal labels are explanatory mathematics, not Photonic syntax. `()` is one empty coherence. An empty file has no coherence and does not represent zero. Parentheses do not box numerals: `(Unit.Unit)` is the same particle as `Unit.Unit`.

This uses one atom for numerical content. There is no extra Zero occurrence, nested record, variable syntax, integer primitive, or numeral-specific runtime behavior. It is minimal within the existing flat, atom-based particle representation, not a claim that every possible encoding has been compared.

<a id="natural-zero-successor-and-equality"></a>

### Zero, successor, and equality

Let N denote the carrier above. Define zero as the class of the empty particle. Define S on N by adjoining one fresh Unit introduction. Freshness makes S well-defined independently of the names already chosen. Define numerical equality as equality of these orderless presentations after anonymous renaming.

**Closure.** Empty is an admitted construction. Extending an admitted finite construction by one step is another admitted finite construction.

**Distinct constructors.** Empty has zero occurrences. Every successor has at least its newly adjoined occurrence. Anonymous renaming and permutation cannot change that distinction.

**Injectivity.** If S(a) and S(b) have equal presentations, remove one Unit from each. Units are indistinguishable in this carrier, so the resulting class is independent of which occurrence is removed. The remaining presentations are a and b; therefore a = b.

**Induction.** Suppose a property holds of zero and is preserved by S. The presentations satisfying it contain the empty construction and are closed under adding one Unit. Least closure therefore includes every numeral in that property. This is a mathematical argument about the specified carrier, not an implemented Photonic quantifier or induction tactic.

**Recursion.** Given an arbitrary set X, an element z of X, and an operation f from X to X, send empty to z and each successor to f of the previous result. Finite construction defines this map everywhere. Removal of one indistinguishable Unit determines a unique predecessor class, so the result is independent of presentation. Induction establishes uniqueness of the map satisfying those two equations.

Zero, successor, their disjointness and injectivity, and the least-closure induction principle give the usual inductive natural-number structure. Lean's reference documents the same mathematical interface for its inductive Nat and its recursion principle. Photonic's representation and execution remain different: the ordinary particle machinery implements our examples, with no arithmetic special cases. [Lean: logical model and Peano axioms](https://lean-lang.org/doc/reference/latest/Basic-Types/Natural-Numbers/#logical-model).

<a id="natural-executing-successor"></a>

### Executing successor

[successor.wave](../mathematics/natural/successor.wave) supplies a control occurrence alongside the numeral two:

```text
Step.Unit.Unit
[Step] Unit
```

The rule consumes Step, preserves the existing numerical remainder, and produces one fresh Unit. The exact result is `Unit.Unit.Unit`. Step is an ordinary program-defined control label, not part of the numeral and not a runtime operator.

Under this one-rule program the same argument applies to any admitted numeral: the input pattern consumes only Step, the remainder is the original numeral, and exactly one fresh Unit is produced. Rust regressions exercise zero through six. Those tests are examples; the preceding argument states the general relationship to S.

<a id="natural-recognizing-the-carrier"></a>

### Recognizing the carrier

To check a candidate flat atom particle p, place exactly one fresh Check beside p in one root coherence and use only these two rules:

```text
[Check] Natural
[Natural.Unit] Natural
```

Ask Obsidian for the exact target `Natural`. Check and Natural are control labels outside the numerical carrier. The fixed rules are the trusted program for this claim; candidate executable rule values or extra declarations are outside this interface.

**Completeness for numerals.** With zero Units, `[Check] Natural` reaches the target. For a successor numeral, the first rule exposes Natural alongside its Units; the second removes one Unit and preserves the marker. Repeating this finite step reaches exactly Natural.

**Soundness for the stated atom interface.** Unknown atoms are never consumed by either rule and remain in the exact target comparison. The total number of Check and Natural markers is preserved. Since the interface supplies one Check, a candidate containing either control label leaves too many markers to equal singleton Natural. The remaining accepted candidates consist entirely of Units, which are precisely the carrier presentations.

This reasoning also respects source inference: every derived Natural has exactly one marker ancestor and zero or more Unit ancestors; Unit itself is never produced by these rules. Projecting an application back to its source can consume several Units at once but cannot consume unknown atoms or combine distinct marker ancestries. It may shorten a path without broadening the accepted atom inputs.

[Membership of two](../mathematics/natural/membership.wave) is a complete runnable instance:

```text
Check.Unit.Unit
[Check] Natural
[Natural.Unit] Natural
```

Obsidian checks reachability under the supplied program. Supplying the target itself as an initial state proves its own reachability; that is why the recognition interface always prefixes Check. Arbitrary code supplied as a candidate could execute and alter the claim. This is a library protocol with an explicit atom-input boundary, not a protected theorem checker.

<a id="natural-quantities-and-resource-sharing"></a>

### Quantities and resource sharing

Numerical equality forgets occurrence spelling inside one numeral. Runtime identity also tracks sharing between coherences and retained environments. Those are different levels of information. The mathematical carrier restricts to plain root particles; it does not erase resource ownership throughout the runtime.

For the existing addition example, independently introduce the operands in separate Add coherences. `[Add, Add] ()` consumes their labels and reunites their unmatched Units. Because the introductions are disjoint, the result has the sum of their multiplicities. If both worlds inherited the same Unit introduction, reunion keeps it once. Such a shared pair is not two independently supplied operands for this addition protocol.

This is the one-generator commutative-monoid view of the representation: empty is the identity and disjoint multiset union combines quantities. It is a mathematical description of the encoding, not a replacement for coherence semantics. [Addition laws](#addition) have written proofs and bounded runtime checks. Universal certificates and further arithmetic remain later work.

<a id="natural-evidence-status"></a>

### Evidence status

| Layer | Established here |
| --- | --- |
| Specification | Carrier, equality, zero, successor, least closure, and input protocol |
| Written mathematical argument | Constructor properties, induction, recursion, recognition soundness/completeness under the stated interface |
| Runtime checks | Zero identity; different numeral sizes; successor and membership for 0 through 6; malformed atom candidates; concrete 2 + 3 |
| Browser | Interactive construction explanation and recorded Rust Obsidian execution graphs |
| Still absent | Object-language universal proofs, a fixed proof-object calculus, independent certificate replay, efficient binary representation |

No finite slider or test collection proves an unbounded theorem. The written specification is the proposed mathematical model; the executable checks test its correspondence with the current runtime. Future work must encode reusable assumptions and proof objects through the original rule model before claiming that Photonic itself has checked these universal arguments.

<a id="addition"></a>

## Addition laws

These are universally stated mathematical laws with written proofs for the carrier in [definition.md](#natural). They are not universally quantified certificates checked inside Photonic. The executable regressions check finite instances and the correspondence between the specified operation and runtime reunion.

<a id="addition-definition-and-operational-contract"></a>

### Definition and operational contract

Represent a and b by finite Unit particles with disjoint introductions. Define a + b as the class of their disjoint union. Renaming either presentation to fresh introductions does not change the result class, so addition is well-defined on numeral classes. This definition also shows closure: concatenating two finite constructions is a finite construction.

The executable protocol supplies exactly two root coherences, one Add with a's Units and another Add with b's Units, under the sole rule `[Add, Add] ()`. The rule consumes both labels and reunites their remainders. Zero operands still occupy their labeled coherences. The exact result is one root coherence representing a + b, including when both operands are zero.

The theorem concerns numerical classes, not equality of arbitrary runtime environments. Shared inherited introductions are outside the operand contract: reunion reconciles them once. Extra code, captured environments, or unrelated atoms are also outside this interface.

<a id="addition-identity"></a>

### Identity

For every a, 0 + a = a and a + 0 = a.

Proof: the empty particle contributes no occurrences to either disjoint union. The resulting presentation represents a.

<a id="addition-successor-compatibility"></a>

### Successor compatibility

For every a and b, S(a) + b = S(a + b) and a + S(b) = S(a + b).

Proof: in either union, precisely one additional fresh Unit is adjoined to the union representing a + b. Its name and position do not affect the numerical class.

These equations together with zero identity characterize addition uniquely. For fixed a, induction on b forces any operation satisfying a + 0 = a and a + S(b) = S(a + b) to agree with this one at every numeral.

<a id="addition-associativity"></a>

### Associativity

For every a, b, and c, (a + b) + c = a + (b + c).

Proof: choose mutually disjoint presentations A, B, and C. Both sides contain exactly their occurrences once. Grouping the union adds no information to the result class. Fresh renaming between stages does not change that class.

<a id="addition-commutativity"></a>

### Commutativity

For every a and b, a + b = b + a.

Proof: exchanging the two disjoint presentations permutes occurrences without changing their multiplicities.

Thus the carrier with addition and zero is a commutative monoid. Together with successor of zero as generator it is the free commutative monoid on one generator: for any commutative monoid M and chosen element u, recursion sends zero to M's identity and each successor to the previous value combined with u. Induction using successor compatibility shows this map preserves addition; induction also proves uniqueness. This statement assumes the ordinary mathematical definition and laws of M, not a new Photonic construct.

<a id="addition-cancellation"></a>

### Cancellation

For every a, b, and c, a + c = b + c implies a = b. By commutativity, c + a = c + b also implies a = b.

Proof by induction on c. At zero the premise is a = b. At S(c), successor compatibility rewrites the premise as S(a + c) = S(b + c). Successor injectivity gives a + c = b + c; the induction hypothesis gives a = b.

<a id="addition-zero-sum"></a>

### Zero sum

For every a and b, a + b = 0 exactly when a = 0 and b = 0.

Proof: two empty presentations have empty union. Conversely, each operand embeds in its disjoint union; if that union is empty, neither operand has an occurrence. Both represent zero. This is an external mathematical implication, not an absence premise in a Photonic rule.

<a id="addition-order-derived-from-addition"></a>

### Order derived from addition

Define a ≤ b to mean there exists a numeral c with a + c = b. This uses a positive witness c. It does not introduce a language comparison operator.

Reflexivity uses c = 0. Transitivity composes witnesses with associativity. For antisymmetry, if a + c = b and b + d = a, then a + (c + d) = a + 0. Cancellation gives c + d = 0; zero sum gives c = d = 0, hence a = b. Totality follows by induction on a and b: zero is below every numeral, and successor compatibility reduces comparison of two successors to their predecessors.

Translation preserves and reflects order: a ≤ b exactly when a + d ≤ b + d. A witness for the first becomes a witness for the second by associativity and commutativity. A witness for the second yields the first by cancellation. Cancellation also makes each difference witness unique.

<a id="addition-evidence-and-scope"></a>

### Evidence and scope

Rust tests execute all pairs from 0 through 4, both associations for triples from 0 through 2, successor compatibility on pairs from 0 through 3, and the 3 + 7 fixture. They also check a wrong target and the shared-introduction counterexample to unrestricted addition. These are finite regression checks, not universal proofs. Cancellation, order, and the monoid mapping property have written proofs here; object-language proof certificates remain future work.

[Multiplication and distributivity](#repetition) have written proofs and an action-based executable protocol. Signed integers and universal proof checking remain separate work. No arithmetic or logical primitive has been added to the runtime.

<a id="repetition"></a>

## Multiplication by repeated addition

A newer [two-runtime-numeral construction](#product) retains the inputs and checks an independent product through Obsidian. The action protocol below remains a separate, simpler representation.

This specification builds on [natural numbers](#natural) and [addition](#addition). It has written general proofs and executable finite reachability checks. It does not introduce runtime arithmetic or universally quantified Photonic proof certificates.

<a id="repetition-definition"></a>

### Definition

For naturals m and n, define m × 0 = 0 and m × S(n) = (m × n) + m. Natural recursion gives existence and uniqueness. Closure follows by induction from closure of addition.

Equivalently, take n independently introduced copies of an m-Unit numeral and form their disjoint union. The zero-copy union is empty; adding one more copy obeys the successor equation. Recursion uniqueness identifies this construction with multiplication. Copies must have fresh introductions; broadcasting a shared numeral and immediately reuniting it would reconcile the same resources once.

<a id="repetition-executable-operand-protocol"></a>

### Executable operand protocol

Represent n by n occurrences of the ordinary control atom Repeat in one root coherence. Represent m by an ordinary action rule that consumes one Repeat and emits m fresh Units. For zero, its output is the existing empty coherence `()` rather than no output. With no repetitions, the input is `()`.

For m = 2 and n = 3:

```text
Repeat.Repeat.Repeat
[Repeat] Unit.Unit
```

The same matcher repeatedly consumes one Repeat and emits the action's Unit particle. This is an action representation of the multiplicand, paired with a repetition representation of the multiplier; it is not a two-Unit-particle multiplication interface. The emitted Unit count is encoded in the rule supplied by the program, not a numeric primitive or a runtime branch. Converting arbitrary runtime numerals into these actions remains unimplemented.

With only this rule, after j direct applications there are n − j Repeat occurrences and m × j Units. Each application strictly decreases the Repeat count and emits fresh Units. At n applications the result is exactly the product. No rule produces Repeat, and the action's input cannot be derived from its Unit outputs, so source inference cannot create extra applications beyond the remaining repetition resources. This establishes termination and the unique terminal numerical result for the stated interface.

Intermediate configurations remain reachable, as usual. “Terminal” is an external description, not a language test for absence. Obsidian checks the exact product configuration; no negative premise is used.

Run [multiplication.wave](../mathematics/natural/multiplication.wave):

```sh
bazel run -c opt //system:command -- obsidian "$PWD/mathematics/natural/multiplication.wave" --target "$PWD/mathematics/natural/six.particle" --cells 32
```

<a id="repetition-an-easy-composed-proof"></a>

### An easy composed proof

[composition.wave](../mathematics/natural/composition.wave) checks (2 × 3) + 1 = 7:

```text
Add.Repeat.Repeat.Repeat,
Add.Unit
[Repeat] Unit.Unit
[Add, Add] ()
```

Both operands use Add because addition is commutative. The two input contexts select distinct coherences; two Add occurrences in one coherence do not satisfy this two-coherence interface.

The multiplication steps may happen before or after reunion. The addition consumes only the Add labels, preserving the three repetitions and the independently supplied Unit. Exactly three repetition applications produce six further Units. Thus either schedule reaches seven. The runtime checks that concrete reachability claim; the preceding invariant explains why the two operations compose.

```sh
bazel run -c opt //system:command -- obsidian "$PWD/mathematics/natural/composition.wave" --target "$PWD/mathematics/natural/seven.particle" --cells 32
```

This is a concrete proof under the supplied program, not a reusable universal equality certificate.

<a id="repetition-the-action-can-be-a-value"></a>

### The action can be a value

[action.wave](../mathematics/natural/action.wave) supplies exactly the same action as a first-class rule value:

```text
([Repeat] Unit.Unit).Repeat.Repeat.Repeat
```

Invocation reads the rule's availability and consumes a Repeat. The rule value remains alongside the six-Unit result. The [target](../mathematics/natural/retained.particle) therefore retains it explicitly:

```text
([Repeat] Unit.Unit).Unit.Unit.Unit.Unit.Unit.Unit
```

```sh
bazel run -c opt //system:command -- obsidian "$PWD/mathematics/natural/action.wave" --target "$PWD/mathematics/natural/retained.particle" --cells 32
```

This demonstrates reusable code as data through the ordinary matcher, without opening an unknown rule or substituting its fields. The result contains code and is not a plain numeral. The declaration form above yields a plain numeral; the value form intentionally keeps its reusable action.

<a id="repetition-first-laws"></a>

### First laws

Let 1 = S(0).

**Right zero:** m × 0 = 0 by definition. **Left zero:** 0 × n = 0 by induction on n, using zero identity of addition.

**Right identity:** m × 1 = (m × 0) + m = m. **Left identity:** 1 × n = n by induction, since adding one is successor.

**Successor on the other operand:** S(m) × n = (m × n) + n. At zero both sides are zero. At S(n), apply the induction hypothesis and regroup the sum using addition associativity, commutativity, and successor compatibility; the result is (m × S(n)) + S(n).

**Commutativity:** m × n = n × m. Induct on n. The zero case uses left zero. The successor case rewrites m × S(n) as (m × n) + m, uses the induction hypothesis, and applies the preceding successor law to S(n) × m.

**Distributivity:** m × (n + p) = (m × n) + (m × p). Induct on p. Zero identity supplies the base case. At a successor, the defining multiplication equation adds m to each side; addition associativity gives the result. Commutativity of multiplication supplies (m + n) × p = (m × p) + (n × p).

**Associativity:** (m × n) × p = m × (n × p). Induct on p. Both sides are zero at zero. At a successor, the left side adds m × n to its previous value; the right side does the same by distributivity and the defining equation. The induction hypothesis identifies their previous values.

Thus the specified natural addition and multiplication form a commutative semiring. These proofs take place in ordinary mathematics over the previously stated carrier. Encoding their universal proof objects remains separate work.

<a id="repetition-evidence-boundary"></a>

### Evidence boundary

Rust regressions exercise declared and value-carried actions for m and n from 0 through 3, wrong numerical targets, commuted products, additive composition, and the single-coherence misuse of Add. They check the operational interface, not unbounded induction. The concrete two-times-three, composed seven, and retained-action fixtures are checked directly. No rule synthesizer, numeral-to-action converter, positional digit system, or generic iterator over arbitrary captured code is claimed.

<a id="repetition-why-repeat-has-not-become-a-general-numeral-multiplier"></a>

### Why Repeat has not become a general numeral multiplier

Changing the name or adding an ordinary counter loop is insufficient. The current program

```text
Tick.Unit.Unit
[Tick.Unit] Tick.Result
```

reaches both Tick.Result.Result by two direct applications and Tick.Result by source inference. The later application's Tick can be witnessed by the first application's output. Projecting that witness consumes the original Tick and both original Units, but the rule emits only one Result. This is permitted by the current source-projection contract; it is not a parser issue or an arithmetic exception.

The action protocol above avoids this particular effect because Repeat never appears in an output. Its fixed Unit output cannot be replaced by an arbitrary supplied numeral merely by changing the trigger label. No generic numeral-to-action conversion has been implemented.

A general construction must address both structural operand reuse and source-projected accumulation. Possible resource-accounting changes must be evaluated against the existing inference and coherence contracts; they must not be silently enabled only for numbers. In particular, requiring complete consumption of a derivation's outputs or retaining its unmatched outputs would change the current semantics. Neither change has been implemented or assumed by these examples.

A [scoped parallel-coherence experiment](verification.md#counterexample) supplied both operands as Unit data and used fixed conversion and copying rules. It was rejected: its preparation scopes can return early, and one times one reaches two Units. The saved source and Obsidian command reproduce that counterexample. A newer [retained-input construction](#product) has positive and negative small-case evidence and a two-times-two witness; its broader soundness and performance obligations remain explicit.

<a id="product"></a>

## Multiplication from two runtime numerals

[product.wave](../mathematics/natural/product.wave) is a fixed ordinary-rule construction over two Unit numerals. The supplied instance is two times two. The rules contain no coefficient-specific Unit output: each copying action emits one fresh Unit, and the number of actions comes from the input data. There is no arithmetic primitive, capture syntax, negative premise, or numeral-specific matcher.

This is a retained-input reachability protocol. Its accepted result contains two archived operands and a Product coherence independent of their resources. It is not the bare expression `Multiply(Unit.Unit,Unit.Unit)` evaluated into a lone numeral. The helper rule values and complete target below are part of the interface. The small cases have closed positive and negative checks; the two-times-two target has a concrete witness, with exploration unfinished. A universal object-language correctness certificate remains unimplemented.

<a id="product-run-the-example"></a>

### Run the example

From the repository root:

```sh
bazel run -c opt //system:command -- obsidian \
  "$PWD/mathematics/natural/product.wave" \
  --target "$PWD/mathematics/natural/archive.particle" \
  --steps 15000000 --states 100000 --records 6000000 \
  --cells 128 --frames 128 --coherences 16
```

Recorded result:

```text
Reached: exact target configuration under the supplied program
Witness s45095: [Archive.Unit.Unit]@root [Archive.Unit.Unit]@root [Unit.Unit.Unit.Unit.Product]@root
45096 configurations; 480987 queued, 0 deferred; exploration unfinished
```

A reached target is positive evidence even while exploration is unfinished. This run does not prove that every other numerical target is unreachable. The search is expensive for such a small calculation; this is a semantic construction, not efficient arithmetic.

For addition, the existing simpler protocol remains:

```text
Add(Unit.Unit.Unit, Unit.Unit.Unit.Unit.Unit.Unit.Unit)
[Add, Add] ()
```

```sh
bazel run -c opt //system:command -- obsidian \
  "$PWD/example/group/addition.wave" --target "$PWD/mathematics/natural/ten.particle"
```

<a id="product-input-and-result"></a>

### Input and result

The first two coherences in product.wave each contain one fixed preparation rule value, a Multiply label, and the supplied Unit occurrences. Change only those two initial Unit counts to supply other operands; leave every rule body unchanged. One helper prepares the factor role and the other prepares the count role. Both input numerals use Unit. For zero, omit the Unit occurrences while retaining the helper and Multiply.

The accepted target for input counts m and n is:

```text
Archive.(m Unit occurrences),
Archive.(n Unit occurrences),
Product.(m × n Unit occurrences)
```

This display describes the protocol; the words and multiplication sign are not Photonic syntax. The actual two-times-two target is:

```text
Archive.Unit.Unit,
Archive.Unit.Unit,
Product.Unit.Unit.Unit.Unit
```

Writing the target as separate initial coherences gives its product occurrences independent introductions. Obsidian compares full canonical states, including sharing, rather than merely counting identical printed labels. Its existing checker supplies this distinction; there is no custom arithmetic target checker.

<a id="product-how-the-rules-work"></a>

### How the rules work

1. Each preparation rule splits its input into an Archive coherence and a working coherence. Their inherited input Units share resource identities.
2. Scoped ordinary rules convert the working Units into X or Y. The distinct internal labels preserve the two operand roles after reunion. A premature return can leave unconverted Units; the language does not test for absence.
3. The main joint rule consumes both preparation rule values as ordinary operands, alongside Factor and Count. This removes the executable preparation capability from its body. Inferred applications at an earlier source cannot keep restarting preparation there through those consumed rule values.
4. A phase consumes a Y and splits into a continuing phase and a copying scope. The copying scope receives the inherited X numeral and converts each X into a fresh Unit. It also removes its inherited Y bookkeeping. Separate scopes give independently computed copies independent introductions.
5. The base cleanup removes the continuing X numeral. Done coherences reunite completed contributions, and the outer return supplies Product. A leaked X, Y, or control label prevents an exact clean numerical target.
6. Two ordinary meta rules remove the known preparation rule values from the Archive coherences. The archived original Unit occurrences remain.

The whole helper rules appear literally in the initial data and in the consuming patterns. Their repetition is visible in the source; no hidden alias expander or structural variable is involved. The runtime uses the existing whole-rule value matcher and lexical capture contract.

<a id="product-why-retain-the-inputs"></a>

### Why retain the inputs?

The earlier preparation attempt could return unconverted Unit occurrences as if they were products. For one times one, that produced an apparent two-Unit result.

Here an unconverted Unit still shares its resource identity with an archived input. It cannot satisfy the target's independent Product occurrences. A fresh Unit comes from an explicit X-to-Unit action. Retaining the original inputs therefore distinguishes copying from simply forwarding an input occurrence, using the same resource identity that already distinguishes broadcast from independent introduction.

Archive and Product are ordinary atoms, not protected types. A Product label alone is not a certificate. Do not erase the archives before checking and then claim the same specification: that would discard the resource-sharing evidence. Arbitrary additional rules can change which targets are reachable, as with every Obsidian claim.

<a id="product-correctness-argument-and-remaining-proof-obligation"></a>

### Correctness argument and remaining proof obligation

A direct successful schedule first converts all working input units, then performs one complete copying scope per Y, then returns the reunited contributions. Each copying scope converts all m X occurrences into m independent Units. There are n such scopes, giving m × n fresh product units. This describes a finite successful schedule for finite operands, assuming sufficient execution resources.

For soundness, incomplete preparation must leave archived sharing; incomplete copying or cleanup must leave bookkeeping; and source-inferred shortcuts must not remove those obligations while dropping numerical contributions. The scope boundaries and consumed preparation code are intended to establish these invariants. The regressions below exercise them, but a complete proof covering every inferred application has not been mechanized. Do not upgrade finite tests to a universal soundness claim.

The full two-times-two example reaches its target. A two-times-three probe, changing only the second initial Unit count and its exact target, returned Unknown after a budget of 100 million work steps, 500,000 states, 12 million records, 128 cells, 128 frames, and 16 coherences with four workers. It recorded 96,180 configurations and 1,607,169 queued tasks, with no deferred applications. Its exploration was unfinished, so this is not evidence of unreachability. Broad validation and search efficiency remain open work before treating this protocol as a mature general arithmetic interface.

<a id="product-compose-with-addition"></a>

### Compose with addition

Supply an additional `Add.Unit` coherence and the ordinary rule:

```text
[Product, Add] Product
```

It consumes the two control labels and reunites their numerical remainders, retaining Product on the sum. For one times one followed by adding one, Obsidian reaches two fresh Product units beside the original archives. The Rust composition regression checks that exact target.

<a id="product-executable-evidence"></a>

### Executable evidence

`system/test/product.rs` uses the same fixed source program and replaces only its initial Unit counts. For both input counts from zero through one, it checks result counts zero, one, and two with closed exploration. Exactly the mathematical product is reached. The test also checks multiplication followed by addition. The larger two-times-two command above is a separately recorded positive witness.

```sh
bazel test //...
```

The runtime gives pending matching and application work a bounded head start over transitive view composition. Both queues remain FIFO; at most 4096 foreground removals occur before a pending composition is serviced. No inference is discarded or given a language-level priority. Queue fairness, budget resumption, chunking, worker-count determinism, and existing semantic reference tests cover the scheduling change.

<a id="product-larger-numbers"></a>

### Larger numbers

1500 × 123 is 184,500. A unary product would require 184,500 Unit occurrences before accounting for archived inputs, intermediate states, or derivation evidence. The current prototype is not practical at that size. The unresolved two-times-three search shows that proof exploration is the immediate bottleneck even before representation size dominates. Compact numeral encodings and a more effective general proof-search strategy are necessary engineering work; increasing limits alone is not an adequate solution.

The [binary library](#representation) demonstrates that alternative: ordinary fixed-width circuit rules and a generic direct-path proof strategy reach 1500 × 123 = 184500. This uses compact numeral data rather than expanding the unary protocol described here.

<a id="word"></a>

## Ternary arithmetic

The default arithmetic tool uses base three for addition, signed subtraction of natural operands, multiplication, and quotient/remainder division. One shared circuit implementation supports ternary and the retained binary benchmark. All execution uses ordinary Photonic rules and the existing runtime.

This is the best measured construction we currently have for the demonstrated workload. It is not a claim of globally optimal arithmetic, arbitrary precision, or a completed functional numeral calculus.

<a id="word-run-all-four-operations"></a>

### Run all four operations

From the repository root:

```sh
bazel run -c opt //mathematics/arithmetic:word -- \
  --operation add --left 1500 --right 123 --expected 1623

bazel run -c opt //mathematics/arithmetic:word -- \
  --operation subtract --left 1500 --right 123 --expected 1377

bazel run -c opt //mathematics/arithmetic:word -- \
  --operation multiply --left 1500 --right 123 --expected 184500

bazel run -c opt //mathematics/arithmetic:word -- \
  --operation divide --left 1500 --right 123 --expected 12 --remainder 24
```

The expected result is a proposed exact target. The external tool encodes it; Photonic proves reachability. The tool does not calculate the answer from the input operands. Unknown means this proof path failed or hit a limit, not that the mathematical claim is false.

| Operation | Decimal result | Ternary result, most significant digit first |
| --- | --- | --- |
| Add | 1623 | 02020010 |
| Subtract | 1377 | 1220000, nonnegative |
| Multiply | 184500 | 00100101002100 |
| Divide | quotient 12, remainder 24 | quotient 0000110, remainder 0000220 |

Leading zeros reflect the complete output width. These displayed digit strings are explanatory notation; they are not new Photonic numerical literals.

<a id="word-the-actual-programs"></a>

### The actual programs

| Operation | Ordinary Photonic source | Exact target |
| --- | --- | --- |
| Add | [program](../mathematics/arithmetic/fixture/ternary/add.wave) | [target](../mathematics/arithmetic/fixture/ternary/add.particle) |
| Subtract | [program](../mathematics/arithmetic/fixture/ternary/subtract.wave) | [target](../mathematics/arithmetic/fixture/ternary/subtract.particle) |
| Multiply | [program](../mathematics/arithmetic/fixture/ternary/multiply.wave) | [target](../mathematics/arithmetic/fixture/ternary/multiply.particle) |
| Divide | [program](../mathematics/arithmetic/fixture/ternary/divide.wave) | [target](../mathematics/arithmetic/fixture/ternary/divide.particle) |

For example, execute the saved multiplication program directly, without the generator:

```sh
bazel run -c opt //system:command -- obsidian \
  "$PWD/mathematics/arithmetic/fixture/ternary/multiply.wave" \
  --target "$PWD/mathematics/arithmetic/fixture/ternary/multiply.particle" \
  --path --steps 20000000 --states 4096 --records 1000000 --cells 4096
```

Add `--directory "$PWD/mathematics/ternary/example"` to a generator command to emit its program and target. A fixed operation and width always generate the same rule bodies; only the initial digit facts change with the operands. Expected values do not influence the program or its width.

<a id="word-readable-digit-rules"></a>

### Readable digit rules

The [small digit library](../library/ternary.particle) makes the arithmetic cases inspectable. Representative complete rules are:

```text
[Function.Sum.0.1.2] Return.([Digit] 0).([Carry] 1)
[Function.Multiply.2.2] Return.([Digit] 1).([Carry] 1)
[Function.Subtract.([Left] 0).([Right] 2).([Borrow] 1)] Return.([Digit] 0).([Borrow] 1)
[Select.LeftOne.RightTwo.ChoiceOne] DigitTwo
```

Three contributions 0 + 1 + 2 give digit zero and carry one. Two times two gives digit one and carry one: 1 + 3 = 4. Subtraction computes a digit and an outgoing borrow; selection chooses a digit from positive choice data. These examples can run independently under the digit library. Their tests cover all input values and permutations of the commutative operations.

The word generator wires the same digit computations using distinct port names. Ports maintain the association between operations in an orderless state. The standalone library is not a constant-size interpreter for arbitrary words, and its commutative patterns do not by themselves route unknown operands through a circuit.

<a id="word-how-the-operations-compose-digit-computations"></a>

### How the operations compose digit computations

**Addition:** put the two input contributions in each column and propagate carries. A local reduction replaces two or three digits by a digit at the same position and a carry at the next. The result has width plus one digits.

**Subtraction:** run borrow chains in both directions. The final borrow selects the nonnegative magnitude; a separate NegativeZero or NegativeOne fact records its sign. Equal inputs give positive zero. Inputs remain natural numbers.

**Multiplication:** multiply every pair of input digits. A product contributes its low digit to column i + j and its carry to column i + j + 1. Reduce the columns with the shared carry machinery. The complete result has twice the input width. Its size suffices because both operands are smaller than 3 to that width; any carry beyond the output width is mathematically zero for valid inputs.

**Division:** process dividend digits from most to least significant. Form T = 3R + the next digit. Attempt subtraction of the divisor twice, keeping a subtraction only when its positive borrow result permits it. The number of successful subtractions is the next quotient digit. If R is initially below a positive divisor, T is below three times the divisor, so two attempts suffice. Intermediate subtraction uses one extra digit; truncation happens only after the quotient digit is resolved.

For a zero divisor, explicit digit rules produce `([Undefined] 1)`, quotient zero, and the original dividend as remainder. That is a total error-payload convention, not a mathematical quotient. For a nonzero divisor the result carries `([Undefined] 0)` and satisfies left = quotient × right + remainder with remainder smaller than the divisor.

```sh
bazel run -c opt //mathematics/arithmetic:word -- \
  --operation subtract --left 123 --right 1500 --expected -1377

bazel run -c opt //mathematics/arithmetic:word -- \
  --operation divide --left 1500 --right 0 \
  --expected 0 --remainder 1500 --undefined
```

<a id="word-notation-and-limits"></a>

### Notation and limits

Photonic's grammar is unchanged. A complete ternary word contains one ordinary DigitNZero, DigitNOne, or DigitNTwo concept at each position. Quotients and remainders use separate prefixes. Explicit zero values are necessary: an omitted output is not accepted as a computed zero. Exact targets include all digits and applicable sign/definedness flags.

The sparse carry library also supports `3^0.3^1.3^1`, representing seven. Each power spelling is an ordinary atom name with meaning supplied by library rules. The caret is not an exponent operator. `Power(3,1)` continues to expand into two coherences; it is not an ordered base/exponent constructor.

The external generator accepts decimal, `0t` ternary, `0b` binary, `0o` octal, and `0x` hexadecimal input, with optional underscores between digits. These forms all encode ternary circuit inputs. For example:

```sh
bazel run -c opt //mathematics/arithmetic:word -- \
  --operation multiply --left 0t2001120 --right 0t11120 --expected 184_500
```

In a `.particle` file, `0t2001120` is just an ordinary concept name. No input-format prefix adds a language operation. The `//mathematics/arithmetic:word` command measures width in ternary digits.

Width is inferred from the largest operand, with a minimum of one, or set explicitly with `--width`. Supported input width is 1 through 20 trits, corresponding to values through 3486784400. Products use up to 40 trits. No output is silently wrapped to fit a smaller target. Larger programs may need more execution resources; this is a finite-width family, not arbitrary precision.

A separately recorded maximum-width command reached 3486784400² = 12157665452083360000 in 1218 events and 1026119 work steps, taking 21839 ms in one local optimized run. That is operational evidence for this case, not a uniform performance bound.

<a id="word-optimizations-and-measurements"></a>

### Optimizations and measurements

The builder propagates possible digit values independently of the actual operands. It emits only applicable truth-table rows: borrow and choice inputs have two values even in a ternary circuit. It omits zero-only product carries from column reduction and removes gates with no path to a requested output. Both radices use the same implementation.

The runtime shares immutable compiled programs across direct-path steps. A scope index selects rules by a required symbol; a further presence check avoids allocating searches that cannot match the current derived view. It uses symbols from the view's target, so inferred abstractions still enable applications at their concrete sources. This is a necessary-condition filter, not a negative language premise. Empty patterns, live rules, captures, multiplicity, and actual multi-coherence matching retain their existing treatment.

Median of five local optimized samples, for inputs 1500 and 123. Timing includes parsing, initialization, path search, and report construction; source construction and final destruction are excluded. Layouts and operations were measured in consecutive batches, so timing may include ordering or thermal effects.

| Operation | Ternary rules | Ternary events | Ternary work | Ternary time | Binary time |
| --- | ---: | ---: | ---: | ---: | ---: |
| Add | 159 | 21 | 1347 | 1352 µs | 4124 µs |
| Subtract | 403 | 36 | 2302 | 3038 µs | 7437 µs |
| Multiply | 1777 | 152 | 19230 | 51359 µs | 500773 µs |
| Divide | 3630 | 268 | 22538 | 73100 µs | 126936 µs |

Ternary won these four comparisons. This does not establish superiority across inputs; the [sparse addition comparison](library.md#structure) includes a case where binary won. Ternary multiplication has more rule rows than binary here, but fewer events and substantially less matching work on the selected path.

Balanced ternary-radix reduction took 89241 µs, with 1795 rules and 39818 work steps for the same multiplication. Column reduction remains the default. Balanced reduction here describes circuit layout, not a signed balanced-ternary digit representation. No parallel speedup is claimed: the path strategy is serial.

```sh
bazel run -c opt //mathematics/arithmetic:benchmark
bazel test -c opt //...
```

<a id="word-what-remains-open"></a>

### What remains open

All pairs of two-trit operands are tested for all four operations, including wrong answers, remainders, and flags. One-trit addition and multiplication also receive exhaustive correct/incorrect target checks. Tests cover larger inputs, maximum-width addition/subtraction, fixed-width rule invariance, and the separate digit library. Runtime tests cover source inference, lexical scope, code values, multi-coherence joins, budgets, scheduling, and irrelevant-rule indexing.

These are concrete checks plus written invariants, not universally quantified arithmetic certificates. Path success proves a witness; a failed path remains Unknown and other paths are not exhausted. Reports are inspection artifacts, not independently replayable certificates.

The [stream investigation](#stream) includes a twelve-rule ternary successor, validated through ordered direct execution traces. The newer [native expression library](expression.md) constructs reusable linked numerals and evaluates signed integer expressions with no fixed digit width. It uses captured scope identities and ordinary declarations. The word implementation remains a separate circuit interface.

<a id="word-library-boundary"></a>

### Library boundary

Circuit and encoding functions return `Result<String, failure::Failure>`. Invalid bases, widths, and values produce structured errors before generation. Circuit operands support binary widths 1 through 32 and ternary widths 1 through 20; output encodings support binary widths 1 through 64 and ternary widths 1 through 40. Power notation supports bases 2 through 16 and normalization widths 0 through 64. Gate tables and representation validation remain private.

These checks validate representation only. Rust emits the program and target description; ordinary rules still perform arithmetic inside the Photonic runtime. The saved program fixtures and execution tests protect that behavior.

<a id="word-source-organization"></a>

### Source organization

`circuit.rs` assembles the arithmetic graph. Its private `circuit/reduce.rs` module reduces digit columns, and `circuit/emit.rs` removes unused gates and writes ordinary Photonic rules. Wire identity comes from the graph’s domain table, so allocation has one source of truth. `encoding.rs` describes targets independently of graph construction.

`test/` holds arithmetic conformance, boundary, and stream checks; `gate/test.rs` checks private truth tables. The private `circuit` and `conformance` filegroups list their inputs explicitly. Adding an implementation or test file requires declaring its build membership.

<a id="word-atomic-ternary-fields"></a>

### Atomic ternary fields

Binary and ternary source and targets use ordinary whole rule values such as `([Port.0] 2)` and `([Digit.0] 2)`. Roles, positions, and payloads are separate concepts; literal numeric symbols replace fused digit names. The private address type stores those components separately. Saved word fixtures and independent expected-value tests use this encoding. Earlier performance tables describe the fused atom encoding; comparison with structural fields requires fresh measurements.

Native local digit operations and stream successor are supplied by [the standard library](library.md#library), using [Photonic Bazel dependencies](build.md#rule). The finite word generator remains a separate host construction tool, not the implementation of that native library.

<a id="stream"></a>

## Ternary streams

Status of this earlier representation: a fixed-rule successor produces the correct digit sequence on the tested direct paths. It does not construct a reusable numeral value. The separate [linked representation](expression.md) now implements reusable numerals, arithmetic, and expression evaluation using existing Photonic semantics.

<a id="stream-representation"></a>

### Representation

Position belongs to the stream, not to a numbered atom. Digits arrive least significant first; End explicitly terminates the input. The mathematical interpretation is:

```text
value(end)         = 0
value(digit, tail) = digit + 3 × value(tail)
```

These equations describe the encoding; they are not Photonic syntax or native operations. Eighteen is 0, 0, 2, End. Twenty is 2, 2, 1, End.

The [example input](../mathematics/ternary/stream.wave) expresses twenty with existing scoped rules:

```text
Read.Carry
[Read] (
    2
    [Next] (
        2
        [Next] (
            1
            [Next] End
        )
    )
)
```

The nested Read/Next definitions are numeral data. Their size grows with the digit count. The arithmetic library is fixed.

<a id="stream-successor"></a>

### Successor

The complete [successor library](../library/stream.particle) contains twelve ordinary rules:

```text
[Carry.0] Copy.([Write] 1)
[Carry.1] Copy.([Write] 2)
[Carry.2] Carry.([Write] 0)

[Copy.0] Copy.([Write] 0)
[Copy.1] Copy.([Write] 1)
[Copy.2] Copy.([Write] 2)

[Carry.End] Finish.([Write] 1)
[Copy.End] Done

[([Write] 0)] Next
[([Write] 1)] Next
[([Write] 2)] Next
[Next.Finish] Done
```

Carry requests an increment. 0 becomes 1; 1 becomes 2. Both switch to Copy, which preserves the remaining digits. 2 becomes 0 and leaves Carry active for the next digit. Carry at End emits a final 1. Every termination rule requires positive evidence.

Each output digit is an ordinary rule value, such as `[Write] 0`. The three meta rules consume those whole values and acknowledge them with Next. This separates emitted digits from input digits without new grammar, numbered ports, numerical primitives, or special matching. There is no Write request in this protocol, so the values are consumed as data.

For twenty, the direct execution emits 0, 0, 2: twenty-one. The result order is in the execution trace. Dotting those three atoms together would discard their order and would not represent twenty-one under this encoding.

<a id="stream-run"></a>

### Run

From the repository root:

```sh
bazel run -c opt //mathematics/ternary:stream -- obsidian \
  --target "$PWD/mathematics/ternary/done.particle" \
  --path --json --steps 100000 --states 1024 --cells 4096 --frames 128
```

The JSON event list includes acknowledgements for 0, 0, 2 in that order. The terminal target is Done; it is not a numeral certificate. The three acknowledgement rules consume the emitted code values, so those values do not remain as a result in the final configuration.

<a id="stream-validation-and-cost"></a>

### Validation and cost

The [regression suite](../mathematics/arithmetic/test/stream.rs) checks every input from zero through 242, unchanged suffixes, leading zeros, carries through one to forty 2 digits, and the successor of the largest unsigned 64-bit integer. It checks the actual acknowledgement sequence against an independent numerical oracle. It also verifies that the twelve arithmetic declarations are unchanged across different input widths and values.

On the tested direct paths, a numeral with n digits takes 3n + 2 events, or 3n + 4 when a final carry appends 1. The twenty-to-twenty-one example takes 11 events and 171 work steps. Forty 2 digits take 124 events and 4934 work steps in the recorded run.

Constant arithmetic code and linear event count do not imply constant memory or linear total execution cost. The forty-digit run retains up to 820 held occurrences across lexical frames. Input syntax depth, retained contexts, matching, canonicalization, and stored path history remain costs. Earlier smaller cell limits changed which successor paths were available; increasing the work budget alone did not address that constraint.

<a id="stream-remaining-contract"></a>

### Remaining contract

This is a direct-path prototype, not a proof that every permitted inference produces one canonical stream. Ancestor Next rules remain lexically available; there is no implicit shadowing or rule priority. Source inference can also bypass intermediate emissions. The language treats those emissions as ordinary states, not externally guaranteed effects. Reaching Done alone therefore cannot prove the output sequence.

The next requirement is to retain an ordered result that another ordinary rule computation can consume, while preserving operand association and accounting for every digit under source inference. Demonstrate that contract for successor before adding two-stream addition, multiplication, or division. Do not introduce a native numeral, hidden binding, or changed parenthesis semantics to fill the gap.

`Power(3, Base.2).Coefficient(2)` remains shared-prefix expansion into two flat coherences. It does not preserve a base/exponent/coefficient record. The recursive direction removes explicit positions, but spelling alone does not supply the missing composition contract.

<a id="algorithm"></a>

## Arithmetic and parallel execution

Status: research direction, with one implemented general evaluator optimization. Shared-prefix grouping is implemented as flat elaboration. A [retained-input two-numeral construction](#product) has bounded execution evidence. The [binary library](#representation) implements compact carry addition and generated fixed-width multiplication; a generic direct-path strategy proves 1500 × 123. Complete-operand rule guards, arbitrary precision, and universal arithmetic certificates remain open. The older action representation remains a separate example.

<a id="algorithm-correctness-precedes-the-numeral-representation"></a>

### Correctness precedes the numeral representation

The current source-inference contract permits a later application to omit evidence-only outputs. Consequently `Tick.Unit.Unit [Tick.Unit] Tick.Result` reaches both one and two Results beside Tick. This is consistent with the existing reference case “Evidence-only co-results are not source leftovers.” It is not a regression introduced by arithmetic.

An experimental projection restriction required every source dependency consumed through inference to be exclusive to the selected witness outputs. It passed 67 of the then-current 68 Rust tests but failed that reference case, removing an accepted configuration. The experiment was reverted. No runtime semantic change is retained from it.

This counterexample does not prove that every general multiplication encoding is impossible. It does show that conventional accumulation loops cannot simply be assumed sound. Establish a fixed ordinary-rule program, with explicit input and result protocols, before optimizing or replacing its numeral representation. Do not add a counter-specific projection exception, implicit negative premise, or theorem-specific evaluator.

<a id="algorithm-compact-data-and-baseline-algorithms"></a>

### Compact data and baseline algorithms

For positional numerals, retain the association between each digit and its position even when storage is orderless. Define decoding into the existing natural carrier, normalization, and equality; then prove each arithmetic transformation respects decoding. Position, bit, and limb notation must be implemented as ordinary data, not new native arithmetic syntax.

Start with carry-based binary addition and schoolbook multiplication as independently checkable baselines. Later compare recursive multiplication algorithms. GMP's manual documents size-dependent choices among basecase, Karatsuba, Toom variants, and FFT algorithms; one method is not optimal at every input size. [GMP multiplication algorithms](https://gmplib.org/manual/Multiplication-Algorithms).

The goal is useful arithmetic performance, not merely a faster unary demonstration. Unary output requires one occurrence per unit of numerical value. Parallel execution cannot remove that representation cost.

<a id="algorithm-use-parallel-coherences-for-independent-subproblems"></a>

### Use parallel coherences for independent subproblems

Karatsuba provides a concrete future acceptance program. Write x = x₀ + Bx₁ and y = y₀ + By₁ for a positional base B. Compute p = x₀y₀, q = x₁y₁, and r = (x₀ + x₁)(y₀ + y₁) in independent coherences. Then combine p + B(r − p − q) + B²q. The middle difference is a natural because r includes p and q; a library can establish it using explicit addition evidence. Subtraction here is mathematical arithmetic, not a negative premise.

This is a known algorithm, not an invented Photonic algorithm. Its independent subproducts are a useful test of the language's parallel-world contract. Immutable operand representations may be shared, while each subcomputation introduces its own result resources. Joining shared input occurrences must not reinterpret them as independently produced output occurrences.

Expose parallel tasks only when their work exceeds coordination cost. Worker counts, task size, retained memory, arithmetic work, and proof-search work must all be measured separately. Existing tiny reference graphs are not evidence of scalable arithmetic speedup.

<a id="algorithm-implemented-remove-redundant-symmetry-orderings"></a>

### Implemented: remove redundant symmetry orderings

The evaluator identifies a conservative class of interchangeable coherences from resource incidence, frame identity, and captures. Canonicalization enumerates one ordering per certified class arrangement while retaining every coherence and every distinct resource. It uses the same semantics for atoms and rule values, with no arithmetic labels involved. See the [proof, benchmark, and limitations](runtime.md#symmetry).

This is an application of graph symmetry reduction. Canonical labeling and graph automorphism algorithms are established work; nauty and Traces are a useful reference point for assessing more general techniques. [Nauty and Traces](https://users.cecs.anu.edu.au/~bdm/nauty/). The implementation here is a small sufficient certificate for swaps, not a replacement for those general algorithms or a novelty claim.

<a id="algorithm-candidate-research-beyond-that-optimization"></a>

### Candidate research beyond that optimization

Investigate representing independent event combinations symbolically rather than enumerating every intermediate configuration. Resource consumption, reads, lexical captures, enabling interactions, and evidence support all contribute to dependence. Disjoint selected footprints alone do not certify independence.

Partial-order reduction is established research for reducing redundant interleavings. [Microsoft Research: transaction-based reduction](https://www.microsoft.com/en-us/research/publication/sound-transaction-based-reduction-without-cycle-detection/). Applying it here would require a Photonic-specific soundness argument. In particular Obsidian can query intermediate configurations: preserving only terminal results would change its contract. A symbolic representation would have to retain the omitted intermediate states' meaning and answer exact reachability correctly.

The possible contribution is a general execution algorithm that combines Photonic's provenance and coherence structure to avoid redundant work. There is no demonstrated new multiplication algorithm or superiority over existing arithmetic libraries yet.

<a id="algorithm-acceptance-sequence"></a>

### Acceptance sequence

1. Establish a correct fixed-program multiplier or explicitly propose the minimum general semantic extension it needs.
2. Resolve how complete-operand evidence participates in ordinary open matching. Shared-prefix syntax elaborates to flat coherences; it does not introduce recursive containers or exact-match guards.
3. Extend the implemented compact addition and fixed-width multiplication with reusable conversion, arbitrary precision, and universally checked correspondence.
4. Compare independent coherence execution with one-worker execution using equivalent programs and equal correctness checks.
5. Compare known arithmetic algorithms across input sizes, including conversion and memory costs.
6. Attempt a new scheduling or arithmetic technique only with an explicit hypothesis, a correctness argument, and reproducible measurements.

These are unfinished milestones. The symmetry optimization is independently useful; it does not complete the arithmetic foundation.

The [representation comparison](#representation) assesses higher radices, native limbs, redundant binary forms, and residue systems. The binary library also implements signed subtraction of unsigned operands and quotient/remainder division with an explicit zero-divisor result. These remain ordinary generated rules with no arithmetic runtime primitive.

The current default is the shared [ternary arithmetic implementation](#word). It includes all four operations, domain-limited digit tables, dead-gate removal, immutable compiled-program sharing, and indexed matching. The recorded comparison favors column reduction over the balanced layout on the tested serial workload. The separate [native expression library](expression.md) now supplies arbitrary finite linked numerals and expression stacks. Universal certificates remain open.

<a id="representation"></a>

## Choosing a compact representation

Research and implementation assessment, 12 September 2026. Historical assessment, followed by implementation updates. The default has since moved to the [ternary word library](#word); binary remains a reference. Decimal conversion can remain an ordinary library concern. A balanced circuit alternative is implemented and measured below. Ternary sparse addition also has mixed measured results; no replacement has been established for all arithmetic operations.

<a id="representation-what-the-runtime-pays-for"></a>

### What the runtime pays for

A shorter numeral is not necessarily a cheaper program. The present circuit library pays for literal rule tables, fan-out particles, configuration canonicalization, and matching work at each event. The serial path strategy repeatedly invokes the same general runtime at a reachable successor. It does not exploit a hardware register, a native word multiply, or concurrent circuit evaluation.

The eleven-bit multiplier uses 1364 rules and 253 events. Its complete output has 22 positions. Unary 184500 would require 184500 occurrences just for the result. That representation reduction is established, but it says little about which compact encoding will minimize matching and evidence costs.

<a id="representation-compare-the-candidates"></a>

### Compare the candidates

| Representation | Potential benefit | Cost in this implementation | Recommendation |
| --- | --- | --- | --- |
| Complete binary | Small positive truth tables; explicit completeness | One atom per bit; carry and borrow dependencies | Keep as baseline |
| Base four or sixteen digit atoms | Fewer data occurrences and digit stages | More literal rule cases per position | Benchmark before adopting |
| Decimal digit atoms | Natural exact decimal input and output | Larger tables and radix conversion | Add when decimal use cases need it |
| Binary limbs | Compact native implementation of large integers | Native limb arithmetic would violate the current library-only contract; opaque atoms need decoding rules | Grouping alone is insufficient |
| Carry-save pairs | Postpone normalization; expose independent local reductions | Extra data, multiple encodings of one value, final normalization | Best next internal experiment |
| Signed digits | More choices for local arithmetic without long carries | More representations and normalization obligations | Candidate after carry-save |
| Residues modulo several bases | Independent component arithmetic | Range bounds, conversion, comparison, and general division | Useful specialized experiment, not a replacement |

The table's Photonic recommendations are engineering inferences, not benchmark results.

<a id="representation-why-larger-digits-do-not-automatically-win"></a>

### Why larger digits do not automatically win

With literal full-subtractor tables, a radix-r digit has r choices for each operand and two incoming borrow values. That is `2r²` rules per position. For approximately w binary bits, there are `w / log₂(r)` positions, ignoring rounding. This simple construction therefore uses approximately `2r²w / log₂(r)` rules, excluding fan-out, selection, and output formatting.

| Radix | Cases per digit | Cases per represented binary bit |
| --- | ---: | ---: |
| 2 | 8 | 8 |
| 3 | 18 | about 11.4 |
| 4 | 32 | 16 |
| 10 | 200 | about 60.2 |
| 16 | 512 | 128 |

This is a derivation for explicit tables, not a lower bound for all possible encodings. Decomposing large digits into small gates avoids the table explosion but restores bit-level work and adds conversion. Smaller state size might still offset larger tables; only matched end-to-end benchmarks can decide.

GMP stores integers as sign and magnitude with arrays of binary limbs. Its single-limb division uses hardware division or multiplication by an inverse. Those mechanisms explain why packed words help a native arithmetic library, but their advantage cannot be assumed for an atom-only evaluator. [GMP integer representation](https://gmplib.org/manual/Integer-Internals), [single-limb division](https://gmplib.org/manual/Single-Limb-Division).

<a id="representation-why-carry-save-is-promising"></a>

### Why carry-save is promising

Carry-save arithmetic retains a number as two contributions and uses independent full-adders to reduce three inputs to two; normalization eventually requires combining the retained contributions. This is an established representation, not a new algorithm. [Parhami, Computer Arithmetic, number representation, slides 47 through 52](https://web.ece.ucsb.edu/Faculty/Parhami/pres_folder/f31-book-arith-pres-pt1.pdf).

Our multiplier already uses local full-adders to compress product columns, then fully resolves each column. A balanced reduction graph that retains two rows until the final boundary could shorten dependencies. Chained arithmetic could also avoid repeatedly normalizing intermediate words. Both changes require an explicit library protocol and measurements; the current serial path strategy may gain little from reduced dependency depth alone.

For Photonic, numerical equivalence is not structural state identity. Two carry-save encodings of the same integer must not be silently merged by runtime canonicalization. A library can normalize them before an exact query, or explicitly prove their equivalence. This preserves the existing deduplication contract.

Independent reductions can be placed in coherences, but transferring operands and collecting results must preserve provenance and completeness. More worlds do not by themselves establish parallel speedup. First compare identical circuits with equivalent targets, then separate scheduler effects from arithmetic representation effects.

<a id="representation-why-residues-are-not-the-next-default"></a>

### Why residues are not the next default

Residue systems support componentwise modular arithmetic but general integer division remains difficult; a recent research paper develops dedicated decompositions for it. This is evidence of a tradeoff, not evidence of superiority for Photonic. [Eric B. Olsen, Direct Integer Division in RNS and its Hardware Solutions (2026 preprint)](https://arxiv.org/abs/2604.04796).

An exact residue library would also need an explicit range below the product of its moduli and a conversion/uniqueness argument. Without those, different integers can have the same residue tuple. Our immediate requirement includes ordinary subtraction, ordered remainders, and division, so canonical binary remains the simpler boundary.

<a id="representation-historical-balanced-comparison"></a>

### Historical balanced comparison

The circuit generator supports both column and balanced reduction layouts, sharing all gate tables and emission logic. The balanced layout reduces triples across columns in rounds, then finishes with the same carry reducer. Both layouts pass arithmetic regressions.

For 1500 × 123 at eleven bits, a local optimized median of five samples measured:

| Layout | Source bytes | Events | Work steps | Time |
| --- | ---: | ---: | ---: | ---: |
| Column | 72460 | 253 | 1551422 | 837769 µs |
| Balanced | 72460 | 253 | 1533076 | 941132 µs |

The balanced version was about 12% slower in this serial sample despite slightly less work. The default remains column reduction. The then-saved column source was the baseline, and both variants prove the same exact target. Parsing, initialization, search, and report construction are timed; source construction and final destruction are excluded. Each layout ran five consecutive samples, so ordering and thermal effects are possible. This is one input and one evaluator, not a general comparison of hardware circuit depth or parallel speed.

```sh
bazel run -c opt //mathematics/arithmetic:benchmark
```

The [ternary comparison](library.md#structure) records four sparse-addition cases, including a counterexample to a universal ternary speedup. The [functional investigation](#stream) records the earlier stream protocol; [native expressions](expression.md) now implement generic arithmetic composition over linked trits.

<a id="representation-next-experiment"></a>

### Next experiment

Compare the current column reducer with a balanced carry-save reduction on the same operand widths, then benchmark base-four subtractor tables. Record source bytes, rules, live particles, events, work, retained evidence, conversion cost, and median wall time. Test the same exact numerical claims and incorrect targets. Keep the general runtime semantics fixed. Adopt another representation only if measured benefits survive its conversion and proof costs.

<a id="decimal"></a>

## Finite decimals

Finite nonnegative decimals can be specified without changing Photonic. This is a mathematical representation proposal with one executable common-scale addition protocol. Arbitrary scale alignment, normalization, decimal parsing, and universal certificates are not implemented.

<a id="decimal-representation"></a>

### Representation

Use a pair of naturals (n, k), with mathematical value n / 10^k. Store n as Unit occurrences in one Coefficient coherence and k as Place occurrences in one Scale coherence. Both labels and Place are ordinary concepts; the two components are explicitly tagged and remain present even when empty.

For example, this represents 0.3:

```text
Coefficient.Unit.Unit.Unit,
Scale.Place
```

Zero at scale zero is `Coefficient, Scale`. These are two labeled coherences, distinct from the one empty coherence representing natural zero. This first interface carries one decimal pair; a collection of pairs needs an explicit association protocol so coefficients and scales cannot be accidentally crossed.

Decimal punctuation and the pair notation in this document are explanatory mathematics, not new Photonic syntax. The runtime treats `0.3` as concepts separated by a dot, not as a decimal literal.

<a id="decimal-numerical-equality"></a>

### Numerical equality

Identify (n, k) and (m, l) when n × 10^l = m × 10^k. These operations initially refer to ordinary mathematical natural arithmetic, whose universally checked Photonic proof library is not yet implemented. Reflexivity and symmetry follow from natural equality. For transitivity, multiply the two equalities by the missing powers and cancel the common positive power of ten. Thus this is an equivalence relation.

In particular (n, k) and (10n, k + 1) represent the same value. Zero at every scale represents the same value. Runtime structural equality does not implement this quotient: Obsidian distinguishes their different coefficient and scale occurrences. Decimal numerical equality will require an explicit library computation and evidence protocol.

A canonical presentation has k = 0 or a coefficient not divisible by ten. Repeated exact division by ten while decreasing positive k terminates because k decreases. If two canonical presentations had different scales, the coefficient at the larger scale would be divisible by ten, a contradiction. At equal scales cross multiplication and cancellation make their coefficients equal. Zero consequently normalizes to (0, 0). This is a mathematical algorithm specification, not an implemented guard or negative premise; a future program must produce explicit quotient/remainder evidence through positive rules.

<a id="decimal-addition"></a>

### Addition

At a common scale k, define (n, k) + (m, k) = (n + m, k). For different scales choose K = max(k, l), align coefficients to n × 10^(K-k) and m × 10^(K-l), then add at K. Raising the chosen common scale multiplies both coefficients by the same power of ten and preserves the represented value. Equivalent input presentations therefore produce equivalent sums. Identity, associativity, commutativity, and cancellation follow from the corresponding natural laws after aligning to one common scale.

The [runnable example](../mathematics/decimal/addition.wave) adds 0.3 and 0.7 at an explicitly shared scale of one:

```text
Add.Unit.Unit.Unit,
Add.Unit.Unit.Unit.Unit.Unit.Unit.Unit,
Scale.Place
[Add, Add] Coefficient
```

The two Add coherences hold independently introduced coefficients. The rule rejoins them as Coefficient; the unrelated Scale coherence stays intact. The [exact target](../mathematics/decimal/result.particle) has ten Units and one Place: 1.0. This program accepts coefficients already at a common scale. It neither verifies two independently supplied scales nor normalizes 1.0 to 1.

```sh
bazel run -c opt //system:command -- obsidian "$PWD/mathematics/decimal/addition.wave" --target "$PWD/mathematics/decimal/result.particle" --cells 32
```

The explicit cell budget accommodates both coefficient occurrences and scale metadata; the default budget of twelve is too small for this example.

The scale must not be broadcast into both operands and accidentally counted twice; keeping one separate Scale coherence makes its ownership explicit. No special metaprogramming is necessary for this concrete operation. Reusable scaling programs remain a future ordinary-rule library task.

<a id="decimal-boundary"></a>

### Boundary

This carrier covers exactly nonnegative fractions with a power-of-ten denominator, not all rationals or reals. Addition and multiplication stay inside it: multiplication uses (nm, k + l). Division is not closed; 1/3 has no finite decimal expansion. Negative values require a signed-number construction. Infinite expansions and real-number completeness require another representation.

Unary coefficients and scales are deliberately transparent but grow quickly. Before building practical decimal arithmetic, extend the [multiplication action protocol](#repetition) to decimal coefficient inputs and implement division with remainder, then implement and verify alignment and normalization. A positional digit encoding could improve efficiency without native number syntax, but its correspondence must be established separately.

<a id="decimal-place-value-in-an-unordered-representation"></a>

### Place value in an unordered representation

Orderless particles preserve multiplicity, but do not infer digit positions. A compact positional representation must explicitly associate each digit with its exponent; a flat pile of Digit and Power labels would lose those associations. The current pair avoids this issue by tagging a single coefficient and its single global scale.

A proposed notation such as `*(3.^(10))` should remain explanatory until an ordinary-rule encoding is demonstrated. It is not a new operator or grammar proposal here. Whole-rule values may provide inert association data under a fixed protocol, but extracting and evaluating arbitrary digit/exponent pairs is not assumed to work automatically. Develop that representation after numeral-to-action conversion and scaling have executable definitions.
