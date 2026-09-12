# Shared prefixes and complete evidence

These examples run through the Rust frontend, runtime, and Obsidian. Parentheses abbreviate repeated prefixes; they introduce no new grammar, container, variable, or arithmetic operation. The integration suite checks every source/target pair listed here, with closed exploration for these finite examples.

## Add three and seven

[addition.wave](addition.wave):

```text
Add(Unit.Unit.Unit, Unit.Unit.Unit.Unit.Unit.Unit.Unit)
[Add, Add] ()
```

The first line expands to two coherences, each carrying Add. The rule consumes their Add occurrences and reunites their ten independently introduced units.

Run from the repository root:

```sh
bazel run //system:command -- obsidian "$PWD/example/group/addition.wave" --target "$PWD/example/group/ten.particle"
```

Expected: `Reached`, with closed exploration. Add `--json` for the source, target, witness, and execution graph, or `--workers 4` to run matching work through the worker pool. Parallel execution preserves the result.

## Match through abstractions

[abstraction.wave](abstraction.wave):

```text
Pair(Seed, Other)
[Seed] Intermediate
[Intermediate] Kind
[Other] Kind
[Pair(Kind, Kind)] ([] Result)
```

Each Kind is an ordinary concept derived by rules. The two operands have different concrete contents and different derivation lengths. The joint rule can apply at their concrete source; its body retains Seed and Other. Obsidian reaches `Result.Seed.Other`.

```sh
bazel run //system:command -- obsidian "$PWD/example/group/abstraction.wave" --target "$PWD/example/group/concrete.particle"
```

This is open pattern matching. Adding Extra to Seed does not prevent that pattern from matching; Extra follows the normal remainder law. It is not a complete-operand check.

## Choose among parallel coherences

[pair.wave](pair.wave):

```text
Pair(A, B, C, D)
[Pair, Pair] Result
```

Every unordered pair is eligible: AB, AC, AD, BC, BD, CD. A selected event consumes two Pair labels and reunites their remainders. The other coherences remain available. Source order establishes no preferred partner.

```sh
bazel run //system:command -- obsidian "$PWD/example/group/pair.wave" --target "$PWD/example/group/selected.particle"
```

This reaches `Result.A.C, Pair.B, Pair.D`. The tests check all six selections. Overlapping matches are alternative events; the existence of six matches does not duplicate each operand into six simultaneous products.

To specify particular independent pairs, [isolated.wave](isolated.wave) uses ordinary distinguishing labels:

```text
First(A, B), Second(C, D)
[First, First] Result
[Second, Second] Result
```

```sh
bazel run //system:command -- obsidian "$PWD/example/group/isolated.wave" --target "$PWD/example/group/separate.particle"
bazel run //system:command -- obsidian "$PWD/example/group/isolated.wave" --target "$PWD/example/group/crossed.particle"
```

The intended pairing is reached; the crossed pairing is unreachable after closed exploration. First and Second are ordinary concepts supplied by this program, not runtime identifiers.

## Check a complete numeral

[natural.wave](natural.wave):

```text
Check.Unit.Unit.Unit
[Check] Number
[Number.Unit] Number
```

The same two rules accept any finite count of Unit, including zero represented by Check alone. Each Unit can be consumed into Number. Check and Number are ordinary concepts; no numeral parser or builtin numerical type participates.

```sh
bazel run //system:command -- obsidian "$PWD/example/group/natural.wave" --target "$PWD/example/group/number.particle"
bazel run //system:command -- obsidian "$PWD/example/group/extra.wave" --target "$PWD/example/group/number.particle"
```

The first reaches exactly Number. The second starts with an additional Extra and cannot reach exactly Number: it can reach Number.Extra. This is an exact positive reachability question about the complete state, not a negative premise in a language rule. A program can explicitly add a rule accounting for Extra if that is the desired definition.

Obsidian is a proof interface, not an implicit rule guard. A later ordinary `[Number] Result` could still act on Number.Extra and preserve Extra. Integrating complete-operand recognition into rule application requires a general boundary contract that the current flat syntax does not express. No hidden Number-specific check was added.

## Preserve resource identity

[broadcast.wave](broadcast.wave):

```text
Seed.X
[Seed] A((), ())
[A, A] ()
```

X is inherited by both outputs of the split. Reunion reaches one X. Conversely, [independent.wave](independent.wave) begins with `A(X,X)`: those are two independent source introductions, and reunion reaches X.X.

```sh
bazel run //system:command -- obsidian "$PWD/example/group/broadcast.wave" --target "$PWD/example/group/shared.particle"
bazel run //system:command -- obsidian "$PWD/example/group/independent.wave" --target "$PWD/example/group/double.particle"
```

Both are reached. The tests also establish that swapping those targets is unreachable. Shared spelling does not erase the distinction between inheritance and independent introduction.

## Verification and remaining work

```sh
bazel test //system:test
```

Tests cover source, pattern, output and target lowering; Cartesian distribution; nesting; empty and repeated alternatives; rule values; scoped-body diagnostics; bounded expansion; runtime provenance; all six pairings; exact evidence; and CLI execution with one or four workers.

A newer [retained-input multiplier](../../mathematics/natural/product.md) now has a two-times-two witness and small closed checks. It requires explicit helper code and an independent-product target; a bare two-argument multiplication interface remains unfinished. Shared-prefix syntax does not turn remainder reunion into a Cartesian arithmetic product. The [design notes](../../document/group.md) retain this boundary rather than treating these successful frontend and evidence examples as a multiplication proof.
