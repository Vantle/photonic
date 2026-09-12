# Natural numbers

A numeral is one coherence containing only repeated `Unit` occurrences. Zero is the existing empty coherence `()`. Successor adds one fresh Unit. This replaces the earlier Zero-marker encoding and uses no new grammar or runtime primitives.

Read the [formal definition](definition.md) or open the [interactive walkthrough](index.html). The walkthrough distinguishes mathematical construction from recorded Rust execution and from future universal proof checking.

## Check two

[membership.lava](membership.lava):

```text
Check.Unit.Unit
[Check] Natural
[Natural.Unit] Natural
```

The interface prefixes exactly one Check to an atom-only candidate and queries exact Natural under these fixed rules. Check and Natural are ordinary control labels outside the numeral. Candidate rules are outside this interface. The definition gives the recognition argument and its trust boundary.

```sh
bazel run -c opt //system/molten/command -- obsidian "$PWD/mathematics/natural/membership.lava" --target "$PWD/mathematics/natural/natural.lava" --json
```

## Compute successor

[successor.lava](successor.lava) turns two into three:

```text
Step.Unit.Unit
[Step] Unit
```

```sh
bazel run -c opt //system/molten/command -- obsidian "$PWD/mathematics/natural/successor.lava" --target "$PWD/mathematics/natural/three.lava" --json
```

[zero.lava](zero.lava) contains `()`. An empty file represents no coherence, which is different from numeral zero.

## Existing addition example

[addition.lava](addition.lava) independently introduces two operands and rejoins their numerical remainders:

```text
Left.Unit.Unit,
Right.Unit.Unit.Unit
[Left, Right] ()
```

```sh
bazel run -c opt //system/molten/command -- obsidian "$PWD/mathematics/natural/addition.lava" --target "$PWD/mathematics/natural/result.lava" --json
```

Its target is five Units. Independence is a precondition: inherited shared Units reconcile once, as usual. General arithmetic laws and an object-language induction proof remain future work. The current formalization covers only natural numbers, with this existing calculation as a compatibility check.

## Reproduce the walkthrough evidence

```sh
bazel run -c opt //mathematics/natural:record > "$PWD/mathematics/natural/evidence.js"
```

The hermetic recorder calls the ordinary parser and Obsidian runtime for every displayed case. Its assertions check the expected verdicts before writing the artifact. It introduces no language functionality.
