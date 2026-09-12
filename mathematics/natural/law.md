# Addition laws

These are universally stated mathematical laws with written proofs for the carrier in [definition.md](definition.md). They are not universally quantified certificates checked inside Photonic. The executable regressions check finite instances and the correspondence between the specified operation and runtime reunion.

## Definition and operational contract

Represent a and b by finite Unit particles with disjoint introductions. Define a + b as the class of their disjoint union. Renaming either presentation to fresh introductions does not change the result class, so addition is well-defined on numeral classes. This definition also shows closure: concatenating two finite constructions is a finite construction.

The executable protocol supplies exactly two root coherences, one Add with a's Units and another Add with b's Units, under the sole rule `[Add, Add] ()`. The rule consumes both labels and reunites their remainders. Zero operands still occupy their labeled coherences. The exact result is one root coherence representing a + b, including when both operands are zero.

The theorem concerns numerical classes, not equality of arbitrary runtime environments. Shared inherited introductions are outside the operand contract: reunion reconciles them once. Extra code, captured environments, or unrelated atoms are also outside this interface.

## Identity

For every a, 0 + a = a and a + 0 = a.

Proof: the empty particle contributes no occurrences to either disjoint union. The resulting presentation represents a.

## Successor compatibility

For every a and b, S(a) + b = S(a + b) and a + S(b) = S(a + b).

Proof: in either union, precisely one additional fresh Unit is adjoined to the union representing a + b. Its name and position do not affect the numerical class.

These equations together with zero identity characterize addition uniquely. For fixed a, induction on b forces any operation satisfying a + 0 = a and a + S(b) = S(a + b) to agree with this one at every numeral.

## Associativity

For every a, b, and c, (a + b) + c = a + (b + c).

Proof: choose mutually disjoint presentations A, B, and C. Both sides contain exactly their occurrences once. Grouping the union adds no information to the result class. Fresh renaming between stages does not change that class.

## Commutativity

For every a and b, a + b = b + a.

Proof: exchanging the two disjoint presentations permutes occurrences without changing their multiplicities.

Thus the carrier with addition and zero is a commutative monoid. Together with successor of zero as generator it is the free commutative monoid on one generator: for any commutative monoid M and chosen element u, recursion sends zero to M's identity and each successor to the previous value combined with u. Induction using successor compatibility shows this map preserves addition; induction also proves uniqueness. This statement assumes the ordinary mathematical definition and laws of M, not a new Photonic construct.

## Cancellation

For every a, b, and c, a + c = b + c implies a = b. By commutativity, c + a = c + b also implies a = b.

Proof by induction on c. At zero the premise is a = b. At S(c), successor compatibility rewrites the premise as S(a + c) = S(b + c). Successor injectivity gives a + c = b + c; the induction hypothesis gives a = b.

## Zero sum

For every a and b, a + b = 0 exactly when a = 0 and b = 0.

Proof: two empty presentations have empty union. Conversely, each operand embeds in its disjoint union; if that union is empty, neither operand has an occurrence. Both represent zero. This is an external mathematical implication, not an absence premise in a Photonic rule.

## Order derived from addition

Define a ≤ b to mean there exists a numeral c with a + c = b. This uses a positive witness c. It does not introduce a language comparison operator.

Reflexivity uses c = 0. Transitivity composes witnesses with associativity. For antisymmetry, if a + c = b and b + d = a, then a + (c + d) = a + 0. Cancellation gives c + d = 0; zero sum gives c = d = 0, hence a = b. Totality follows by induction on a and b: zero is below every numeral, and successor compatibility reduces comparison of two successors to their predecessors.

Translation preserves and reflects order: a ≤ b exactly when a + d ≤ b + d. A witness for the first becomes a witness for the second by associativity and commutativity. A witness for the second yields the first by cancellation. Cancellation also makes each difference witness unique.

## Evidence and scope

Rust tests execute all pairs from 0 through 4, both associations for triples from 0 through 2, successor compatibility on pairs from 0 through 3, and the 3 + 7 fixture. They also check a wrong target and the shared-introduction counterexample to unrestricted addition. These are finite regression checks, not universal proofs. Cancellation, order, and the monoid mapping property have written proofs here; object-language proof certificates remain future work.

[Multiplication and distributivity](multiplication.md) now have written proofs and an action-based executable protocol. Signed integers and universal proof checking remain separate work. No arithmetic or logical primitive has been added to the runtime.
