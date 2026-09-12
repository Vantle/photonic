# Frontend contract

The frontend parses source structure independently of evaluation. It replaces the monorepo's seek-based, byte-at-a-time constructor with a [pest](https://docs.rs/pest/2.9.1/pest/) grammar. The reference is [Molten's language document](https://github.com/Vantle/Vantle/blob/0b693aa583e71c60a225cbb3b4cd53ecfbbaf9fb/Molten/document/molten.page.rs) and its [constructor](https://github.com/Vantle/Vantle/blob/0b693aa583e71c60a225cbb3b4cd53ecfbbaf9fb/Molten/system/graph/symbolic/constructor.rs).

## Representation

`parser::parse(&str)` returns a tree borrowing the original UTF-8 source. Nodes occupy one flat vector in source preorder. Every node has a kind, a half-open byte range, and its parent's vector index. The root is a module spanning the entire input. The returned vector is read-only, and dropping a tree does not recursively drop nested children.

Concepts retain their original spelling through source spans. No string is allocated per label. Whitespace, dots, commas, groups, and source contexts remain distinct. Group and context spans include their delimiters. Source order is preserved even though runtime particles will be orderless.

The library does not own the source or copy it on success. A command attaches the source to a diagnostic only on failure. Node indices belong to one tree and are not runtime identities.

## Grammar boundary

| Form | Syntax node |
| --- | --- |
| Nonempty run excluding delimiters, dot, comma, and ASCII whitespace | Concept |
| `.` | Continuation |
| `,` | Coherence |
| `( ... )` | Group |
| `[ ... ]` | Context |
| ASCII space, tab, CR, LF, vertical tab, form feed | Space |

Concepts preserve Unicode exactly; normalization and a stricter identifier alphabet are not introduced. Non-ASCII whitespace remains concept text, following the old ASCII whitespace boundary. There is no comment, string, keyword, or numeric-literal syntax at this layer.

The parser consumes the entire source. Missing, extra, and mismatched delimiters produce errors, including zero-width spans at end of input. Empty modules and empty groups are structurally valid. Empty coherences and repeated continuation tokens are preserved: the old fixtures exercise empty coherences, and their semantic meaning must be decided during lowering. Consequently `A..B`, `[A,,]`, and a standalone context can parse without establishing that they are valid executable expressions.

Whitespace is retained because the existing constructor distinguishes whitespace from continuation. It must not be silently discarded before the rule-sequencing and scope semantics are settled.

A per-input scan rejects nesting beyond 128 levels before entering the generated recursive parser. This is an implementation resource limit, not a claim about the language's eventual limits. No global parser setting is changed. Large flat input is not constrained by this depth limit. Input-size and allocation budgets remain future work.

## Verification

Tests cover nesting and parentage, repeated concepts, Unicode spans, CRLF preservation, malformed delimiters, end-of-input diagnostics, the accepted depth boundary, rejected excessive depth, and wide flat input. Structural acceptance is intentionally tested separately from semantic execution.

Future lowering will consume the tree into symbols, particles, and scoped rules, with separate semantic diagnostics. A later editor frontend can add recovery or an incremental tree without coupling runtime state to parser-library types.

## Executable reference representation

The JavaScript plan executes an explicit structured representation independently of this source parser. An atom is a string; a whole rule value is `{rule: {input, output, negative?}}`. Output `{particle: [...]}` returns values; `{body: [...]}` enters a nested body, optionally with explicit particle values. Whole rule values capture their lexical environment when created.

This settles the value-versus-body distinction in the runtime reference without pretending that the bracket abbreviation `[[A,B]] ([B])` already lowers to it. The first executable text grammar should require an explicit output for a complete rule value and distinguish returning code from entering a body. Text lowering and its source-level acceptance tests remain unimplemented.
