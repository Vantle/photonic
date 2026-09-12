# Frontend contract

Molten has one grammar. The pest-backed `parser::parse` preserves the original source tree; `lowering::parse` interprets that same tree as an executable program. Lowering does not maintain another grammar or reserve additional characters or keywords.

The boundary follows the [original language document](https://github.com/Vantle/Vantle/blob/0b693aa583e71c60a225cbb3b4cd53ecfbbaf9fb/Molten/document/molten.page.rs): concepts, dots, commas, source contexts, groups, and ASCII whitespace. The interpretation of nested rules below makes the previously incomplete executable boundary explicit; it is not a claim that the historical constructor already executed every form.

## Expressions

| Form | Example | Meaning |
| --- | --- | --- |
| Concept | `True` | One atom |
| Particle | `A.B` | Orderless occurrences in one coherence |
| Coherences | `A, B` | Independent initial coherences |
| Rule | `[A] B` | Consume a matching source and produce B |
| Joint rule | `[A, B] (C, D)` | Join two inputs and produce two coherences |
| Scope | `[Enter] ([Payload] Result)` | Enter a body with a local rule |
| Produced rule | `[Seed] [A] B` | Produce the rule from A to B as one live value |
| Rule operand | `[[A] B] C` | Match a whole rule value and replace it with C |
| Initial rule value | `([A] B).A` | A live rule and an A occurrence |
| Empty particle | `()` | One empty coherence |
| Empty source | `[] A` | Zero operand positions, requiring an execution site |
| No result | `[A]` | No output coherences |
| Empty result | `[A] ()` | One output coherence, retaining unmatched remainder |

```text
And.True.False.Extra
[True] Boolean,
[False] Boolean,
[And.Boolean.Boolean] (
    [True.True] True,
    [True.False] False,
    [False.False] False,
)
```

Dots combine particle members; commas separate coherences. At module or body level, a source context starts a declaration. Commas before another declaration or at the end of a scope delimit declarations without creating an empty initial coherence. Explicit `()` creates an empty coherence. Whitespace separates expressions; it does not replace a dot inside an input particle.

A rule's following expression supplies its output. A following source context is itself a rule value: `[Seed] [A] B` constructs code. After a completed output, a new source context starts the next declaration. Use commas to terminate a rule with no output before another declaration: `[A], [B] C`.

An output group containing declarations enters their scope. A group of plain concepts only groups those concepts; it does not allocate a named record or introduce a frame. Multiple grouped destinations such as `[A] (B) (C)` produce separate coherences. A body can currently initialize at most one explicit coherence; this is a representation limitation diagnosed during lowering.

A nested context in a particle is a complete rule value, including the empty-output case. Thus `[[A,B]] ([B])` consumes the rule whose two inputs are A and B and whose output is empty, then enters a body containing the consuming rule `[B]`. It is not an alias declaration. For ordinary whole-rule replacement, write `[[A] B] [A] C`. See [generalization](generalization.md).

## No additional syntax

There are no variable sigils, quote operators, named constructors, arrows, semicolon terminators, braces, or absence keywords. The characters in `$x`, `@`, `->`, `;`, `{`, `}`, and the word `unless` are ordinary concept text wherever the original delimiter rules permit them. They have no special execution behavior. Old extended programs must be migrated; they are not interpreted by a compatibility grammar.

`Box(A.B)` groups A and B alongside Box; it does not construct one opaque Box value. `$x` matches the literal concept `$x`, not an arbitrary value. `@([A] B)` includes an ordinary @ concept. Parentheses and brackets retain their original delimiter roles.

## Running source

```sh
bazel run //system/molten/command -- run "$PWD/example/conjunction.lava"
bazel run //system/molten/command -- run "$PWD/example/replacement.lava" --json
bazel run //system/molten/command -- parse "$PWD/example/decoherence.lava"
bazel test //system/molten/test
```

The CLI also accepts a structured JSON representation of the same positive program model. It has no negative-premise field, variable form, or named constructor form. Unknown fields are rejected rather than silently changing program meaning. `Not`, `True`, and `False` are ordinary concepts, not keywords or built-in logic.

## Representation and diagnostics

The lossless tree borrows UTF-8 source and stores nodes in a flat vector with kinds, byte spans, and parent indices. Lowering builds child adjacency once and reads those nodes. Runtime values are atoms or complete rule values; there are no variable or named-structure variants, including through JSON.

Structural parsing preserves empty groups, repeated dots, and empty coherence positions. Executable lowering additionally checks expression composition. For example, `A..B` is structurally representable but has a missing operand. Missing or mismatched delimiters, unsupported body initialization, and excessive nesting have structured diagnostics. Delimiter nesting and executable nesting are each limited to 128 levels. Executable nesting also counts consecutive rule contexts, even when their brackets are shallow. These are implementation resource bounds.

Tests cover the original alphabet, Unicode spans, grouping, nested rule production and replacement, the historical bracket example, original-style Boolean declarations, empty results, malformed composition, and runtime reachability. They do not establish that every previously unresolved historical expression has a unique intended semantics.
