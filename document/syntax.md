# Frontend contract

Molten has two pest-backed frontends. `lowering::parse(&str)` produces an executable `source::Program`; `parser::parse(&str)` preserves source structure for inspection. Structural acceptance does not promise execution. Both report byte spans through structured diagnostics.

## Executable native syntax

A program starts with an optional initial configuration, followed by rule definitions. Each initial configuration and definition ends with a semicolon. Dots combine values within a particle; commas separate coherences. Every rule has bracketed input coherences and an explicit `->` output.

```text
And.True.False.Extra;
[True] -> Boolean;
[False] -> Boolean;
[And.Boolean.Boolean] -> {
    [True.True] -> True;
    [True.False] -> False;
    [False.False] -> False;
};
```

The body receives concrete operands through the [binding contract](binding.md). It can also specify one initial particle before its local definitions. Multiple explicit initial coherences inside one body are rejected by the current representation; multiple output bodies remain supported.

| Form | Native example | Meaning |
| --- | --- | --- |
| Initial configuration | `A.X, B.Y;` | Two independently evolving coherences |
| Many-to-many rule | `[A, B] -> C, D;` | Two input positions and two output coherences |
| Empty particle | `();` | One empty initial coherence |
| Empty input | `[] -> A;` | No operand positions; application still needs an execution site |
| Empty output | `[A] -> [];` | No output coherences |
| Empty output particle | `[A] -> ();` | One output coherence, retaining unmatched remainder |
| Negative premise | `[Start] unless [Q] -> P;` | Conditional application depending on absence evidence |
| Named structure | `Box(A.B)` | One value containing an orderless particle |
| Complete-value variable | `[Box($value)] -> Wrapped($value);` | Extract and construct through ordinary matching |
| Whole rule value | `@([A] -> B)` | One value, capturing its lexical environment when created |
| Nested body | `[Enter] -> { Make; [Make] -> Result; };` | Enter a body with explicit Make and local definitions |

An empty program has no execution site. To explore a zero-input rule, provide an initial coherence such as `();`. Whole-rule constructors require complete definitions, without a semicolon inside `@(...)`. For example:

```text
@([A] -> B).A;
[@([A] -> B)] -> @([A] -> C);
```

`[[A,B]] ([B])` is not executable native syntax. Input-only brackets do not construct a complete rule value. The native grammar distinguishes returned code from entered bodies explicitly; `$name` captures one complete value, and `Box(A.B)` constructs a named orderless structure. These forms also work inside rule patterns and constructors; see [structural values](structure.md).

Atoms preserve Unicode and exclude ASCII whitespace, syntax delimiters, `@`, semicolons, and `->`. `$` is reserved for variable names. A bare `$` is invalid. `Box()` is an empty named structure; `Box(A.B)` contains two orderless members. There is no comment, quoting, or string-literal syntax. Whitespace between grammar tokens is ignored. Native nesting is limited to 128 levels before recursive parsing; this is an implementation limit. Malformed syntax, excessive nesting, and unsupported body initialization have distinct diagnostic variants.

## Running source

From the repository root:

```sh
bazel run //system/molten/command -- run "$PWD/example/conjunction.lava"
bazel run //system/molten/command -- run "$PWD/example/capture.lava" --workers 4 --records 1000000 --json
bazel run //system/molten/command -- run "$PWD/example/reference.json" --format json --json
bazel run //system/molten/command -- parse "$PWD/example/decoherence.lava"
bazel test //system/molten/test
```

`run` infers JSON input from a `.json` extension; `--format molten` or `--format json` overrides that choice. `--json` selects the output report format independently. The older `decoherence.lava` fixture exercises structural parsing; use the native examples for execution. See [runtime](runtime.md) for exploration budgets and library resumption.

## Structured executable representation

`source::Program` holds initial particles and definitions. Values are atoms, `{variable: "name"}` patterns, `{structure: "Box", particle: [...]}` structures, or `{rule: {input, output, negative?}}` constructors. Output `{particle: [...]}` returns values; `{body: [...]}` enters a body, optionally with explicit particle values. Native lowering produces this representation, and the CLI can also deserialize it from JSON. The JavaScript kernel accepts the earlier ground subset; native structural traces appear alongside it in the plan.

Constructors can build code from bound complete values. Instantiated content and nested captures participate in identity; generated code is not limited to a precompiled catalog. Native source and JSON use the same Rust runtime without a separate type evaluator.

## Structural representation

`parser::parse(&str)` returns a tree borrowing the original UTF-8 source. Nodes occupy one flat vector in source preorder. Every node has a kind, a half-open byte range, and its parent's vector index. The root is a module spanning the entire input. The returned vector is read-only, and dropping a tree does not recursively drop nested children.

Concepts retain their original spelling through source spans. No string is allocated per label. Whitespace, dots, commas, groups, and source contexts remain distinct. Group and context spans include their delimiters. Source order is preserved even though runtime particles are orderless.

The library does not own the source or copy it on success. A command attaches the source to a diagnostic only on failure. Node indices belong to one tree and are not runtime identities.

## Structural grammar boundary

| Form | Syntax node |
| --- | --- |
| Nonempty run excluding delimiters, dot, comma, and ASCII whitespace | Concept |
| `.` | Continuation |
| `,` | Coherence |
| `( ... )` | Group |
| `[ ... ]` | Context |
| ASCII space, tab, CR, LF, vertical tab, form feed | Space |

Concepts preserve Unicode exactly; normalization and a stricter identifier alphabet are not introduced. Non-ASCII whitespace remains concept text, following the old ASCII whitespace boundary. There is no comment, string, keyword, or numeric-literal syntax at this layer.

The parser consumes the entire source. Missing, extra, and mismatched delimiters produce errors, including zero-width spans at end of input. Empty modules and empty groups are structurally valid. Empty coherences and repeated continuation tokens are preserved: the old fixtures exercise empty coherences, and executable validity is checked by the separate native grammar. Consequently `A..B`, `[A,,]`, and a standalone context can parse without establishing that they are valid executable expressions.

Whitespace is retained for source fidelity. Native lowering instead uses explicit semicolons to delimit initial configurations and definitions.

A per-input scan rejects nesting beyond 128 levels before entering the generated recursive parser. This is an implementation resource limit, not a claim about the language's eventual limits. No global parser setting is changed. Large flat input is not constrained by this depth limit. Input-size and allocation budgets remain future work.

## Verification

Tests cover nesting and parentage, repeated concepts, Unicode spans, CRLF preservation, malformed delimiters, end-of-input diagnostics, the accepted depth boundary, rejected excessive depth, and wide flat input. Structural acceptance is intentionally tested separately from semantic execution.

The executable grammar lowers directly into `source::Program`, independently of this lossless tree. A later editor frontend can add recovery or an incremental tree without coupling runtime state to parser-library types.
