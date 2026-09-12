# Natural numbers

The initial experiment represents naturals as `Zero` or a single `Successor(number)`. This describes the intended input domain; it is not yet an implemented membership checker.

[addition.lava](addition.lava) computes two plus three using ordinary structural rules. Its recursion is:

```text
add(Zero, right) = right
add(Successor(left), right) = add(left, Successor(right))
```

Each step transfers one successor from the left operand to the right. For finite well-formed numerals, the left operand decreases until it is zero. The resulting value is `Successor(Successor(Successor(Successor(Successor(Zero)))))`.

```sh
bazel run -c opt //system/molten/command -- run "$PWD/mathematics/natural/addition.lava" --json
```

Run this command from the repository root. To check the exact result with [Obsidian](../obsidian.md):

```sh
bazel run -c opt //system/molten/command -- obsidian "$PWD/mathematics/natural/addition.lava" --target "$PWD/mathematics/natural/result.lava" --json
```

The execution report includes intermediate configurations and direct source-inferred applications, so its event graph can have edges that skip individual recursive steps. The arithmetic example has no negative premises or competing computational rules.

The recursive call stays at the outermost value. Molten does not automatically apply arbitrary rules inside every constructor: writing an unevaluated addition inside `Successor(...)` would require explicit evaluation rules. This example needs no such contextual evaluation convention.

The base rule returns its bound right-hand value. Without membership evidence it would also return an arbitrary non-numeral there. We make no type-safety or natural-number recognition claim for this two-rule experiment.

The checked example reports `reached`, with witness `s3`, four configurations, and a closed exploration. A regression also checks that the same program cannot reach the numeral four as its complete target configuration. These are concrete reachability checks relative to the two supplied addition rules.

Next, define membership evidence and equality checking, followed by quantification and induction. Associativity and commutativity remain theorem targets, not conclusions established by the example.
