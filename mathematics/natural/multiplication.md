# Multiplication by repeated addition

A newer [two-runtime-numeral construction](product.md) retains the inputs and checks an independent product through Obsidian. The action protocol below remains a separate, simpler representation.

This specification builds on [natural numbers](definition.md) and [addition](law.md). It has written general proofs and executable finite reachability checks. It does not introduce runtime arithmetic or universally quantified Photonic proof certificates.

## Definition

For naturals m and n, define m × 0 = 0 and m × S(n) = (m × n) + m. Natural recursion gives existence and uniqueness. Closure follows by induction from closure of addition.

Equivalently, take n independently introduced copies of an m-Unit numeral and form their disjoint union. The zero-copy union is empty; adding one more copy obeys the successor equation. Recursion uniqueness identifies this construction with multiplication. Copies must have fresh introductions; broadcasting a shared numeral and immediately reuniting it would reconcile the same resources once.

## Executable operand protocol

Represent n by n occurrences of the ordinary control atom Repeat in one root coherence. Represent m by an ordinary action rule that consumes one Repeat and emits m fresh Units. For zero, its output is the existing empty coherence `()` rather than no output. With no repetitions, the input is `()`.

For m = 2 and n = 3:

```text
Repeat.Repeat.Repeat
[Repeat] Unit.Unit
```

The same matcher repeatedly consumes one Repeat and emits the action's Unit particle. This is an action representation of the multiplicand, paired with a repetition representation of the multiplier; it is not a two-Unit-particle multiplication interface. The emitted Unit count is encoded in the rule supplied by the program, not a numeric primitive or a runtime branch. Converting arbitrary runtime numerals into these actions remains unimplemented.

With only this rule, after j direct applications there are n − j Repeat occurrences and m × j Units. Each application strictly decreases the Repeat count and emits fresh Units. At n applications the result is exactly the product. No rule produces Repeat, and the action's input cannot be derived from its Unit outputs, so source inference cannot create extra applications beyond the remaining repetition resources. This establishes termination and the unique terminal numerical result for the stated interface.

Intermediate configurations remain reachable, as usual. “Terminal” is an external description, not a language test for absence. Obsidian checks the exact product configuration; no negative premise is used.

Run [multiplication.wave](multiplication.wave):

```sh
bazel run -c opt //system:command -- obsidian "$PWD/mathematics/natural/multiplication.wave" --target "$PWD/mathematics/natural/six.particle" --cells 32
```

## An easy composed proof

[composition.wave](composition.wave) checks (2 × 3) + 1 = 7:

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

## The action can be a value

[action.wave](action.wave) supplies exactly the same action as a first-class rule value:

```text
([Repeat] Unit.Unit).Repeat.Repeat.Repeat
```

Invocation reads the rule's availability and consumes a Repeat. The rule value remains alongside the six-Unit result. The [target](retained.wave) therefore retains it explicitly:

```text
([Repeat] Unit.Unit).Unit.Unit.Unit.Unit.Unit.Unit
```

```sh
bazel run -c opt //system:command -- obsidian "$PWD/mathematics/natural/action.wave" --target "$PWD/mathematics/natural/retained.wave" --cells 32
```

This demonstrates reusable code as data through the ordinary matcher, without opening an unknown rule or substituting its fields. The result contains code and is not a plain numeral. The declaration form above yields a plain numeral; the value form intentionally keeps its reusable action.

## First laws

Let 1 = S(0).

**Right zero:** m × 0 = 0 by definition. **Left zero:** 0 × n = 0 by induction on n, using zero identity of addition.

**Right identity:** m × 1 = (m × 0) + m = m. **Left identity:** 1 × n = n by induction, since adding one is successor.

**Successor on the other operand:** S(m) × n = (m × n) + n. At zero both sides are zero. At S(n), apply the induction hypothesis and regroup the sum using addition associativity, commutativity, and successor compatibility; the result is (m × S(n)) + S(n).

**Commutativity:** m × n = n × m. Induct on n. The zero case uses left zero. The successor case rewrites m × S(n) as (m × n) + m, uses the induction hypothesis, and applies the preceding successor law to S(n) × m.

**Distributivity:** m × (n + p) = (m × n) + (m × p). Induct on p. Zero identity supplies the base case. At a successor, the defining multiplication equation adds m to each side; addition associativity gives the result. Commutativity of multiplication supplies (m + n) × p = (m × p) + (n × p).

**Associativity:** (m × n) × p = m × (n × p). Induct on p. Both sides are zero at zero. At a successor, the left side adds m × n to its previous value; the right side does the same by distributivity and the defining equation. The induction hypothesis identifies their previous values.

Thus the specified natural addition and multiplication form a commutative semiring. These proofs take place in ordinary mathematics over the previously stated carrier. Encoding their universal proof objects remains separate work.

## Evidence boundary

Rust regressions exercise declared and value-carried actions for m and n from 0 through 3, wrong numerical targets, commuted products, additive composition, and the single-coherence misuse of Add. They check the operational interface, not unbounded induction. The concrete two-times-three, composed seven, and retained-action fixtures are checked directly. No rule synthesizer, numeral-to-action converter, positional digit system, or generic iterator over arbitrary captured code is claimed.

## Why Repeat has not become a general numeral multiplier

Changing the name or adding an ordinary counter loop is insufficient. The current program

```text
Tick.Unit.Unit
[Tick.Unit] Tick.Result
```

reaches both Tick.Result.Result by two direct applications and Tick.Result by source inference. The later application's Tick can be witnessed by the first application's output. Projecting that witness consumes the original Tick and both original Units, but the rule emits only one Result. This is permitted by the current source-projection contract; it is not a parser issue or an arithmetic exception.

The action protocol above avoids this particular effect because Repeat never appears in an output. Its fixed Unit output cannot be replaced by an arbitrary supplied numeral merely by changing the trigger label. No generic numeral-to-action conversion has been implemented.

A general construction must address both structural operand reuse and source-projected accumulation. Possible resource-accounting changes must be evaluated against the existing inference and coherence contracts; they must not be silently enabled only for numbers. In particular, requiring complete consumption of a derivation's outputs or retaining its unmatched outputs would change the current semantics. Neither change has been implemented or assumed by these examples.

A [scoped parallel-coherence experiment](../../document/multiplication/README.md) supplied both operands as Unit data and used fixed conversion and copying rules. It was rejected: its preparation scopes can return early, and one times one reaches two Units. The saved source and Obsidian command reproduce that counterexample. A newer [retained-input construction](product.md) has positive and negative small-case evidence and a two-times-two witness; its broader soundness and performance obligations remain explicit.
