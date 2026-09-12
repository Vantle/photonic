# Beginning the mathematical library

## Recommendation

Begin with natural numbers as a concrete experiment, while developing the minimum judgment and proof machinery needed to certify their laws. Do not attempt all number systems or select an entire foundational system before completing this first example of proof checking.

The working direction is constructive: evidence establishes a proposition through explicit rules. Classical principles, if introduced later, should be visible assumptions. This is a proposed library design, not a restriction on Molten programs or a commitment that the runtime implements dependent type theory.

Inductive natural numbers supply a useful precedent: constructors determine their recursion and induction principles. Lean's documentation presents this relationship explicitly. We can adopt the mathematical discipline without adopting Lean's runtime representation or built-in primitives. [Induction and recursion](https://docs.lean-lang.org/theorem_proving_in_lean4/induction_and_recursion.html), [natural numbers](https://lean-lang.org/doc/reference/latest/Basic-Types/Natural-Numbers/).

## Obsidian comes first

[Obsidian](obsidian.md) establishes concrete supported reachability relative to supplied initial states and rules. Its first implementation compares exact canonical configurations and returns reached, unreachable, or unknown. Use it to check the addition example now; build mathematical certification on top of an explicitly chosen calculus. Portable proof replay and quantified reachability are subsequent milestones.

## First milestones

| Stage | Deliverable | Acceptance |
| --- | --- | --- |
| 0. Obsidian | Exact concrete reachability with an inspectable witness report | Exact identity, consuming/source-inferred events, conditional support, and budget uncertainty are preserved; no independent certificate claim |
| 1. Concrete naturals | `Zero`, `Successor(number)`, and an ordinary addition program | Concrete sums reach the expected numeral; inputs remain intact in historical configurations; no extra numerical operands are invented |
| 2. Judgments and evidence | A written grammar for objects, propositions, assumptions, and proof objects | Every proposed proof rule has explicit premises and conclusion; discovery alone is never an acceptance criterion |
| 3. Natural membership and equality | Evidence for numeral membership; reflexivity, symmetry, transitivity, and constructor congruence | A designated checker accepts valid finite certificates and fails to establish malformed ones; malformed objects are not silently classified as naturals |
| 4. Binding and induction | Object-level variable representation, capture-avoiding substitution, quantification, and a natural induction rule | One checked certificate establishes a statement for arbitrary naturals; a finite enumeration of examples cannot stand in for induction |
| 5. Addition laws | Left zero identity, successor law, right zero identity, associativity, and commutativity | Each theorem records its statement, assumptions, and checked proof; the operational addition example agrees with the specification |
| 6. Multiplication and order | Recursive multiplication, distributivity, and an explicit definition of natural order | Derived laws reuse the preceding library; no theorem-specific runtime support |

The first universal theorem target is `add(Zero, n) = n`, directly reflecting the base definition once quantification and equality are available. The first inductive target is `add(m, Successor(n)) = Successor(add(m, n))`; it then supports a proof of `add(n, Zero) = n`. The current executable example establishes only particular calculations.

## Decisions to settle through the first proof

1. **What is a judgment?** Specify separately that an object is a natural number, that two objects are equal, and that evidence establishes a proposition. These can be ordinary structured data; a label alone has no authority.
2. **What is trusted?** Fix the allowed axioms and inference rules, the certificate checker, and how its inputs and accepted conclusion are identified. Unrestricted user rules must not be able to impersonate successful checking merely by emitting the same label. Initially run a fixed checker program over inert certificate data, without importing candidate executable rules. Record the trusted program and assumptions with the result.
3. **How does equality work?** Separate runtime structural identity, explicitly chosen computational equality, and mathematical equality established by evidence. An arbitrary Molten rule `A -> B` is not automatically an axiom that mathematical objects A and B are equal. Compatible substitution and constructor congruence must be justified by the chosen calculus.
4. **How are binders represented?** Use explicit object-language binder and variable data with a specified substitution operation. First-class Molten rules can retain their own binders, but arbitrary binder-syntax extraction is not implemented. Compare named variables with freshness against positional binding before freezing the representation; do not silently equate mathematical variables with runtime pattern variables.
5. **How are assumptions reused?** Ordinary mathematical hypotheses can usually be reused, while runtime occurrences are consumable. Represent assumption lookup and proof reuse explicitly inside the calculus. Do not change coherence ownership or use broadcast as an implicit logical contraction rule.
6. **What does unsuccessful search mean?** Suspended exploration is unknown. Absence of a discovered proof is not mathematical negation. Start certification with positive, finite proof objects; represent negation through the chosen logical rules, not runtime `unless` by default.

A small trusted checker is an established architecture: Lean distinguishes elaboration from checking by its kernel. In Molten, evidence construction and checking may themselves use ordinary computation. The trust distinction describes which rules justify a theorem; it need not introduce a new runtime phase. [Elaboration and compilation](https://lean-lang.org/doc/reference/latest/Elaboration-and-Compilation/).

## The first numerical representation

Use the metamathematical grammar `number ::= Zero | Successor(number)`. A numeral is one nested value, not a particle containing interchangeable unit occurrences. `Successor(Zero)` denotes one; `Successor(Successor(Zero))` denotes two. Only a single well-formed child qualifies as a successor numeral.

This is an initial representation choice. Unary naturals make structural arguments transparent, but they are not an efficient representation of large integers. After laws and representation independence are established, define binary numerals and prove a conversion correspondence using the same language mechanisms. No primitive integer optimization is required for this experiment.

Use named fields for operations: `Add(Left(number).Right(number))`. Particle order is irrelevant, but Left and Right preserve operand roles. This scales to operations that are not commutative and avoids deriving mathematical commutativity from a storage convention.

## Beyond the first milestones

Develop reusable logical connectives, functions, relations, and finite collections alongside arithmetic. Choose representations of equivalence relations and quotients before constructing integers from pairs of naturals or rationals from fractions. Then develop algebraic structures and homomorphisms; later address reals, limits, and analysis with their required completeness and logical assumptions explicit.

These are dependencies to investigate, not a promise that all mathematics fits an already settled foundation. We should compare candidate foundational encodings after equality, binding, and induction have been demonstrated. Independence and undecidability should remain visible limits of a selected theory, rather than reasons to alter computation semantics.

## Working method

Keep executable examples small and reproducible with Bazel. Add semantic acceptance tests when the relevant interface is stable. A bounded collection of successful executions is regression evidence, not a universal proof. For the first theorem checker, seek adversarial malformed certificates and independently compare its accepted inference rules with an established formal account.

A milestone is complete only when its definition, assumptions, evidence format, and verification procedure agree. A proof should remain checkable without trusting the search strategy that found it. The current JSON runtime reports are inspection artifacts, not yet independently replayable mathematical proof certificates.
