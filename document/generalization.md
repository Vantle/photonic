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

Establish encodings of binding, reusable hypotheses, and induction using rule composition before claiming a universal mathematical foundation. If an operation cannot yet be encoded, record that gap. Do not introduce an implicit wildcard convention, a reserved concept masquerading as an ordinary label, or a theorem-specific evaluator.

All rules require positive evidence. Negative premises are not a missing feature or a future encoding target: the language does not define them. A user-defined concept named Not is allowed, with exactly the behavior its program supplies. Meta rules and abstractions can compress explicit definitions; they do not create default behavior from missing evidence.

The [natural-number examples](../mathematics/natural/README.md) demonstrate a useful immediate simplification: unary quantities and addition can use occurrence multiplicity and ordinary coherence reunion. They require no record constructors or pattern variables.
