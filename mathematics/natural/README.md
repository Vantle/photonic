# Natural numbers

Use `Zero` with n occurrences of `Successor` to represent the numeral n. For example, two is `Zero.Successor.Successor`. The particle is orderless, but occurrences retain multiplicity. A numeral has exactly one Zero and no unrelated concepts.

This replaces the constructor-based experiment. Parentheses do not construct nested numerical records, and the runtime has no numerical primitives.

## Membership

[membership.lava](membership.lava) is the first concrete proof program:

```text
Zero.Successor.Successor
[Zero] Natural
[Successor.Natural] Natural
```

A sequential path is `Zero.Successor.Successor` to `Natural.Successor.Successor` to `Natural.Successor` to `Natural`. Source inference can also establish direct applications at earlier configurations. These are alternative configurations, not accumulated numerical operands.

The rules recognize any finite numeral of this representation by consuming one successor at a time. The example checks the particular numeral two. Reaching exact Natural leaves no unaccounted-for occurrences. An input with an unknown base cannot reach that target under these rules. This is not a protected certificate interface: supplying Natural itself already satisfies that reachability target. A future checker must specify its input boundary and keep candidate data from impersonating acceptance.

```sh
bazel run -c opt //system/molten/command -- obsidian "$PWD/mathematics/natural/membership.lava" --target "$PWD/mathematics/natural/natural.lava" --json
```

## Addition

[addition.lava](addition.lava) uses independent coherences for the two operands:

```text
Left.Zero.Successor.Successor,
Right.Zero.Successor.Successor.Successor
[Left.Zero, Right.Zero] Zero
```

The joint rule consumes each operand's label and zero marker. Ordinary remainder reunion carries the two plus three independently introduced successor occurrences into the result. One fresh Zero marks the resulting numeral five.

```sh
bazel run -c opt //system/molten/command -- obsidian "$PWD/mathematics/natural/addition.lava" --target "$PWD/mathematics/natural/result.lava" --json
```

The independence condition matters. If operands share an inherited successor introduction, reunion reconciles it once. Shared histories therefore do not represent two independently supplied quantities for this encoding. The library must record this precondition rather than change coherence semantics to force arithmetic behavior.

Regression tests check exact numeral five and exclude four. Membership and addition are concrete reachability experiments relative to these fixed programs. They are not yet independently certified universal theorems, an equality calculus, or a formal induction principle.

## Next

Specify proof objects and reusable assumptions using the original rule expressions. Establish binding and induction encodings before claiming universally checked arithmetic laws. Unary occurrence counts make the first experiment small; efficient binary arithmetic and representation correspondence remain future work.
