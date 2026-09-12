# Natural numbers

This is a mathematical specification of the first Molten number representation, with written arguments and executable finite checks. The ambient reasoning assumes finite constructions, equality, and induction over those constructions. It is not yet a foundational calculus or a universally quantified proof checked inside Molten.

## Carrier

A numeral occupies one root coherence. Its particle contains only occurrences of the ordinary concept `Unit`. All occurrences have distinct introductions within that coherence. There are no captured values, local frames, or unrelated concepts in the representation.

Start with the empty particle. Repeatedly add one fresh Unit, finitely many times. Admit exactly the results of these constructions. Identify presentations that differ only in occurrence names or order. Do not identify different multiplicities.

This is a least-closure definition: it admits no extra elements beyond those generated from the empty particle. Merely declaring an arbitrary zero and a successor operation would not exclude additional elements unrelated to zero.

| Mathematical notation | Molten representation |
| --- | --- |
| 0 | `()` |
| S(0) | `Unit` |
| S(S(0)) | `Unit.Unit` |
| S(S(S(0))) | `Unit.Unit.Unit` |

The notation S and the decimal labels are explanatory mathematics, not Molten syntax. `()` is one empty coherence. An empty file has no coherence and does not represent zero. Parentheses do not box numerals: `(Unit.Unit)` is the same particle as `Unit.Unit`.

This uses one atom for numerical content. There is no extra Zero occurrence, nested record, variable syntax, integer primitive, or numeral-specific runtime behavior. It is minimal within the existing flat, atom-based particle representation, not a claim that every possible encoding has been compared.

## Zero, successor, and equality

Let N denote the carrier above. Define zero as the class of the empty particle. Define S on N by adjoining one fresh Unit introduction. Freshness makes S well-defined independently of the names already chosen. Define numerical equality as equality of these orderless presentations after anonymous renaming.

**Closure.** Empty is an admitted construction. Extending an admitted finite construction by one step is another admitted finite construction.

**Distinct constructors.** Empty has zero occurrences. Every successor has at least its newly adjoined occurrence. Anonymous renaming and permutation cannot change that distinction.

**Injectivity.** If S(a) and S(b) have equal presentations, remove one Unit from each. Units are indistinguishable in this carrier, so the resulting class is independent of which occurrence is removed. The remaining presentations are a and b; therefore a = b.

**Induction.** Suppose a property holds of zero and is preserved by S. The presentations satisfying it contain the empty construction and are closed under adding one Unit. Least closure therefore includes every numeral in that property. This is a mathematical argument about the specified carrier, not an implemented Molten quantifier or induction tactic.

**Recursion.** Given an arbitrary set X, an element z of X, and an operation f from X to X, send empty to z and each successor to f of the previous result. Finite construction defines this map everywhere. Removal of one indistinguishable Unit determines a unique predecessor class, so the result is independent of presentation. Induction establishes uniqueness of the map satisfying those two equations.

Zero, successor, their disjointness and injectivity, and the least-closure induction principle give the usual inductive natural-number structure. Lean's reference documents the same mathematical interface for its inductive Nat and its recursion principle. Molten's representation and execution remain different: the ordinary particle machinery implements our examples, with no arithmetic special cases. [Lean: logical model and Peano axioms](https://lean-lang.org/doc/reference/latest/Basic-Types/Natural-Numbers/#logical-model).

## Executing successor

[successor.lava](successor.lava) supplies a control occurrence alongside the numeral two:

```text
Step.Unit.Unit
[Step] Unit
```

The rule consumes Step, preserves the existing numerical remainder, and produces one fresh Unit. The exact result is `Unit.Unit.Unit`. Step is an ordinary program-defined control label, not part of the numeral and not a runtime operator.

Under this one-rule program the same argument applies to any admitted numeral: the input pattern consumes only Step, the remainder is the original numeral, and exactly one fresh Unit is produced. Rust regressions exercise zero through six. Those tests are examples; the preceding argument states the general relationship to S.

## Recognizing the carrier

To check a candidate flat atom particle p, place exactly one fresh Check beside p in one root coherence and use only these two rules:

```text
[Check] Natural
[Natural.Unit] Natural
```

Ask Obsidian for the exact target `Natural`. Check and Natural are control labels outside the numerical carrier. The fixed rules are the trusted program for this claim; candidate executable rule values or extra declarations are outside this interface.

**Completeness for numerals.** With zero Units, `[Check] Natural` reaches the target. For a successor numeral, the first rule exposes Natural alongside its Units; the second removes one Unit and preserves the marker. Repeating this finite step reaches exactly Natural.

**Soundness for the stated atom interface.** Unknown atoms are never consumed by either rule and remain in the exact target comparison. The total number of Check and Natural markers is preserved. Since the interface supplies one Check, a candidate containing either control label leaves too many markers to equal singleton Natural. The remaining accepted candidates consist entirely of Units, which are precisely the carrier presentations.

This reasoning also respects source inference: every derived Natural has exactly one marker ancestor and zero or more Unit ancestors; Unit itself is never produced by these rules. Projecting an application back to its source can consume several Units at once but cannot consume unknown atoms or combine distinct marker ancestries. It may shorten a path without broadening the accepted atom inputs.

[Membership of two](membership.lava) is a complete runnable instance:

```text
Check.Unit.Unit
[Check] Natural
[Natural.Unit] Natural
```

Obsidian checks reachability under the supplied program. Supplying the target itself as an initial state proves its own reachability; that is why the recognition interface always prefixes Check. Arbitrary code supplied as a candidate could execute and alter the claim. This is a library protocol with an explicit atom-input boundary, not a protected theorem checker.

## Quantities and resource sharing

Numerical equality forgets occurrence spelling inside one numeral. Runtime identity also tracks sharing between coherences and retained environments. Those are different levels of information. The mathematical carrier restricts to plain root particles; it does not erase resource ownership throughout the runtime.

For the existing addition example, independently introduce the operands in separate Left and Right coherences. `[Left, Right] ()` consumes their labels and reunites their unmatched Units. Because the introductions are disjoint, the result has the sum of their multiplicities. If both worlds inherited the same Unit introduction, reunion keeps it once. Such a shared pair is not two independently supplied operands for this addition protocol.

This is the one-generator commutative-monoid view of the representation: empty is the identity and disjoint multiset union combines quantities. It is a mathematical description of the encoding, not a replacement for coherence semantics. General arithmetic proofs remain later work.

## Evidence status

| Layer | Established here |
| --- | --- |
| Specification | Carrier, equality, zero, successor, least closure, and input protocol |
| Written mathematical argument | Constructor properties, induction, recursion, recognition soundness/completeness under the stated interface |
| Runtime checks | Zero identity; different numeral sizes; successor and membership for 0–6; malformed atom candidates; concrete 2 + 3 |
| Browser | Interactive construction explanation and recorded Rust Obsidian execution graphs |
| Still absent | Object-language universal proofs, a fixed proof-object calculus, independent certificate replay, efficient binary representation |

No finite slider or test collection proves an unbounded theorem. The written specification is the proposed mathematical model; the executable checks test its correspondence with the current runtime. Future work must encode reusable assumptions and proof objects through the original rule model before claiming that Molten itself has checked these universal arguments.
