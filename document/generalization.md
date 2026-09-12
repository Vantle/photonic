# Generalization through rules

Use the same source expression at every level. A concept can derive an abstraction; a whole rule can derive an abstraction; either can be consumed or carried into a body by the existing projection contract. There is no special grammar for types, quotation, or metavariables.

## Abstraction preserves its concrete witness

```text
Not.True
[True] Boolean
[False] Boolean
[Not.Boolean] (
    [True] False,
    [False] True,
)
```

The derivation of Boolean enables the Not rule at the concrete source. Its body receives True, and its local rule returns False. Boolean is evidence for this application, not an extra operand carried alongside True. The same mechanism applies when the concrete witness is a whole rule value.

## Repeated abstractions accept different concrete operands

```text
Pair.Seed,
Pair.Other
[Seed] Intermediate
[Intermediate] Kind
[Other] Kind
[Pair.Kind, Pair.Kind] ([] Result)
```

This executable example reaches Result.Seed.Other. The two Kind occurrences are ordinary literal patterns, enabled by the program's derivations. Seed and Other need not be equal. Their evidence can require different numbers of steps. The joint application can open its body at the concrete source, preserving Seed and Other; the local rule adds Result and returns. Removing Other's derivation prevents this application. Putting both operands in one coherence also prevents this two-coherence match.

`system/test/obsidian.rs` checks these claims, including operand permutation. This example demonstrates abstraction and concrete witness preservation across flat coherences. It does not implement nested groups, copy either operand, or compute multiplication.

The intended `Multiply(Number,Number)` abbreviates `Multiply.Number, Multiply.Number`; it does not introduce a private operand container. There are no implicit variables, no requirement that both numbers be equal, and no builtin numerical type. The distinction between partial pattern matching and evidence for a complete operand is recorded in [shared-prefix groups](group.md).

## Code is another value

```text
Seed.A
[Seed] [A] B
```

Seed produces the rule from A to B. That rule is locally executable where it is present. Invoking it reads its availability; matching it as an operand consumes it. Actual execution reaches the live rule alongside B; source inference also enables Seed.B. Both alternatives retain their existing support and ownership semantics.

```text
([A] B).A
[[A] B] [A] C
```

The outer rule replaces the complete A-to-B rule with an A-to-C rule. The produced code executes through the ordinary runtime. This is local replacement, not global equivalence or an alias that retroactively changes earlier events. Whole-rule matching compares normalized code and its lexical capture under the ground matching contract.

## What this establishes

Nested brackets express rule values without an @ operator. Ordinary derivations express abstraction without a $ variable. Groups express scoped rule bodies without braces. All three use the existing matcher, source projection, event support, and canonical configuration machinery.

This is deliberately smaller than arbitrary structural substitution. It does not provide a generic operation that opens any unknown rule, binds its input fields, and rebuilds different code. The former variable/constructor extension and its implementation have been removed, not renamed or hidden in JSON. A rule can carry unknown concrete information into a body through source inference and remainder transfer; that does not automatically give it arbitrary structural field access.

The finite ground code shapes come from the program. Runtime activation, replacement, and captured environments evolve, but this implementation does not synthesize arbitrary new code shapes from metavariables. A finite code catalog does not guarantee a finite state graph: multiplicity, coherences, and retained captures can still grow.

## Remaining research

Operand abstraction must come from ordinary rule derivations. The user clarified that extending general rule semantics does not authorize explicit capture forms, variable conventions, or a new binding meaning for existing punctuation. The experimental capture matcher was removed. `Multiply(Number,Number)` is intended as shared-prefix shorthand whose Number occurrences require rule-derived evidence, not implicit variables.

Establish encodings of binding, reusable hypotheses, and induction using rule composition before claiming a universal mathematical foundation. If an operation cannot yet be encoded, record that gap. Do not introduce an implicit wildcard convention, a reserved concept masquerading as an ordinary label, or a theorem-specific evaluator.

All rules require positive evidence. Negative premises are not a missing feature or a future encoding target: the language does not define them. A user-defined concept named Not is allowed, with exactly the behavior its program supplies. Meta rules and abstractions can compress explicit definitions; they do not create default behavior from missing evidence.

The [natural-number examples](../mathematics/natural/README.md) demonstrate a useful immediate simplification: unary quantities use only Unit multiplicity, with empty as zero; addition uses ordinary coherence reunion. They require no record constructors or pattern variables.

The [shared-prefix investigation](group.md) records the implemented elaboration of original expressions such as `A(B,C)` into flat coherences. It supersedes the earlier nested-container proposal. Whole-operand guards within ordinary rules remain unresolved; a [retained-input multiplication construction](../mathematics/natural/product.md) now supplies both counts as data, with bounded validation and explicit remaining proof obligations; shared spelling alone supplies neither.
