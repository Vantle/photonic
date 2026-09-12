# Molten

Computational expression over hypergraphs. This repository starts the standalone language core, using the build conventions from [Vantle Registry](https://github.com/Vantle/registry).

A **rule** describes computation, including abstraction derivations; an **event** records one application. Types are computation in the same iterative model. A **coherence** is an independently evolving parallel context. **Divergence** creates several coherences; **decoherence** combines compatible coherences. See the [core design](document/core.md) and [terminology](document/terminology.md) for the distinction between these language concepts and ordinary data operations.

Implemented:

- A generated parser with lossless source structure, UTF-8 byte spans, and diagnostic errors.
- Canonical particles with multiplicity-preserving matching and union.
- Pure rule application to an explicit binding, broadcasting the unmatched context to every output.

The parser and rule kernel are separate APIs. `parser::parse` produces `syntax::Tree` and structured `failure::Failure` diagnostics. `particle::Particle` and `rule::Rule` form the independent rewrite kernel. The named crate root, `system/molten/molten.rs`, exposes these modules directly without re-exports. CLI argument handling and execution live in `system/molten/command`; parser, particle, and rule checks run as separate integration test targets. Source lowering, graph storage, inference, polymorphism, and execution are not implemented yet. The [runtime proposal](document/runtime.md) records the unresolved semantics and the next implementation slice. The [interactive research report](document/research.html) compares causal interaction models and exposes a duplication counterexample in the old broadcasting algebra. The [interactive runtime plan](document/plan.html) executes joint source inference, generated rule values, lexical captures, whole-rule abstractions, nested bodies, and conditional support in a finite JavaScript reference. It includes 26 programs and an interactive matching gate. The [earlier program laboratory](document/program.html) preserves the separate experiments, and the [implementation sequence](document/plan.md) records the remaining decisions and work. These are offline design experiments, separate from the language implementation.

## Build

Install **Bazel 9.2.0**. No separately installed Rust, Cargo, or native compiler is required.

```sh
bazel build //...
bazel test //...
bazel run //:format -- --check
bazel run //system/molten/command -- parse "$PWD/example/decoherence.lava"
```

The `parse` command prints the concrete syntax tree; success establishes structural validity, not executability. Both `.lava` and `.magma` use the same parser. See the [frontend contract](document/syntax.md).

Bazel downloads Rust 1.98.1, hermetic LLVM, platform SDKs, and crates. Initial fetching needs network access. Compilation runs without network access. Cargo describes dependencies; Bazel owns compilation and testing. Both lockfiles are checked in, and normal commands reject stale Bazel resolution data.

```sh
bazel run //:format
bazel run //:update --config=refresh
bazel mod deps --config=refresh
bazel test //... --config=refresh
bazel test //...
```

Review both lockfiles after updates. Developer commands use the downloaded Rust tools. Bazel creates no convenience symlinks in the checkout; use `bazel info bazel-bin` to locate outputs.

The toolchain configuration targets ARM64 and x86-64 macOS, GNU Linux, and GNULLVM Windows. Platform support requires native runtime tests, not just successful cross-compilation. The Windows host override and optional remote execution configuration follow Registry. Put local executor settings in ignored `user.bazelrc`.

## Rule kernel

`particle::Particle<Concept>` stores an immutable, sorted multiset. `Concept` can be a borrowed label today and a compact interned symbol during lowering. Construction sorts once; remainder and union use linear merges. Duplicate concepts remain significant.

`rule::Rule::apply` accepts one particle per input pattern, already arranged in pattern order. It returns `None` if the arity or any multiset match fails. On success it returns every output with the concatenated remainder added. It preserves duplicate outputs and does not mutate its input. An empty result is a successful rule with no outputs.

This is a provisional implementation of the old documented rewrite algebra. Literal broadcasting can multiply carried context across divergence/decoherence cycles; its final resource semantics remains under discussion. The caller must eventually establish distinct occurrence identities, coherence compatibility, and lineage eligibility before invoking it; it does not select graph nodes or authorize their interaction.

Rules themselves implement structural equality, ordering, and hashing, so the same generic `Rule::apply` can replace whole nested rule values. The JavaScript reference additionally activates produced rule values locally and tracks their capture and read support. The Rust kernel does not execute the graph, and nested source syntax still needs lowering.

The crates supply parsing ([pest](https://docs.rs/pest/2.9.1/pest/)), error derivation ([thiserror](https://docs.rs/thiserror/)), diagnostics ([miette](https://docs.rs/miette/)), and argument handling ([clap](https://docs.rs/clap/)). Molten implements its own multiset rewrite semantics. Runtime storage dependencies will be selected with the identity model.
