# Natural numbers

A numeral is one coherence containing only repeated `Unit` occurrences. Zero is the existing empty coherence `()`. Successor adds one fresh Unit. This replaces the earlier Zero-marker encoding and uses no new grammar or runtime primitives.

Read the [formal definition](definition.md) or open the [interactive walkthrough](index.html). The walkthrough distinguishes mathematical construction from recorded Rust execution and from future universal proof checking.

## Check two

[membership.wave](membership.wave):

```text
Check.Unit.Unit
[Check] Natural
[Natural.Unit] Natural
```

The interface prefixes exactly one Check to an atom-only candidate and queries exact Natural under these fixed rules. Check and Natural are ordinary control labels outside the numeral. Candidate rules are outside this interface. The definition gives the recognition argument and its trust boundary.

```sh
bazel run -c opt //system:command -- obsidian "$PWD/mathematics/natural/membership.wave" --target "$PWD/mathematics/natural/natural.particle" --json
```

## Compute successor

[successor.wave](successor.wave) turns two into three:

```text
Step.Unit.Unit
[Step] Unit
```

```sh
bazel run -c opt //system:command -- obsidian "$PWD/mathematics/natural/successor.wave" --target "$PWD/mathematics/natural/three.particle" --json
```

[zero.particle](zero.particle) contains `()`. An empty file represents no coherence, which is different from numeral zero.

## Existing addition example

[addition.wave](addition.wave) independently introduces two operands and rejoins their numerical remainders:

```text
Add.Unit.Unit,
Add.Unit.Unit.Unit
[Add, Add] ()
```

```sh
bazel run -c opt //system:command -- obsidian "$PWD/mathematics/natural/addition.wave" --target "$PWD/mathematics/natural/result.particle" --json
```

Its target is five Units. Independence is a precondition: inherited shared Units reconcile once, as usual. [Addition laws](law.md) have written mathematical proofs and finite runtime checks. Universal object-language proofs remain future work.

## Reproduce the walkthrough evidence

```sh
bazel run -c opt //mathematics/natural:record > "$PWD/mathematics/natural/evidence.js"
```

The hermetic recorder calls the ordinary parser and Obsidian runtime for every displayed case. Its assertions check the expected verdicts before writing the artifact. It introduces no language functionality.

## Addition laws and 3 + 7

[Addition laws](law.md) specifies disjoint-union addition and proves identity, successor compatibility, associativity, commutativity, cancellation, zero sum, and additive order in ordinary mathematics. Finite Rust regressions check the runtime correspondence; universal Photonic certificates remain future work.

[sum.wave](sum.wave) adds three and seven Units. From the repository root, check its exact ten-Unit result:

```sh
bazel run -c opt //system:command -- obsidian "$PWD/mathematics/natural/sum.wave" --target "$PWD/mathematics/natural/ten.particle"
```

The command reports Reached and displays the ten-Unit witness. Use `run` instead of `obsidian` and omit `--target` to inspect all reachable configurations.

[Finite decimals](../decimal/README.md) extends the mathematical representation to a coefficient and scale, with a runnable common-scale addition example.

## Multiplication and composition

[Multiplication](multiplication.md) uses repeated ordinary actions: `[Repeat] Unit.Unit` adds two fresh Units per Repeat. One operand is an action, not a second Unit particle. The document states the representation boundary, proves the first multiplication laws in ordinary mathematics, and demonstrates the action as a reusable rule value.

Check the composed calculation (2 × 3) + 1 = 7:

```sh
bazel run -c opt //system:command -- obsidian "$PWD/mathematics/natural/composition.wave" --target "$PWD/mathematics/natural/seven.particle" --cells 32
```

Both addition inputs use Add. The matcher selects distinct coherences, so commutativity needs no left/right role labels.

## Two Unit operands

The [retained-input multiplication construction](product.md) supplies both operands as Unit data and uses fixed rules, scoped copying, and whole-rule consumption. Its two-times-two example reaches four fresh Product units beside the archived inputs. Small positive and negative cases close completely; larger search and a universal soundness proof remain open. This protocol does not replace the bare numeral interface with an undocumented builtin.
