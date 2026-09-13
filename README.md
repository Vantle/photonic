# Photonic

**[Read the interactive webbook](index.html)** — philosophy, grammar, features, the Rust runtime, and mathematics in one page. Open `index.html` locally to use its illustrations and recorded execution explorers offline.

Both `.particle` and `.wave` contain the same Photonic source syntax. By convention, `.particle` holds reusable rules or data and `.wave` holds a runnable script. The extension does not change evaluation. Structured programs and execution reports remain JSON.

Computational expression over hypergraphs, with a native Rust frontend and runtime. The hermetic build follows [Vantle Registry](https://github.com/Vantle/registry).

The [mathematics library](mathematics/README.md) begins with a [natural-number chapter](index.html#natural), a written formal definition and [Obsidian](mathematics/obsidian.md), which checks concrete state reachability. Explicit mathematical proof checking, equality, and induction are planned.

A **rule** describes computation, including abstraction derivations; an **event** records one application. Types are computation in the same iterative model. A **coherence** is an independently evolving parallel context. **Divergence** creates several coherences; **decoherence** combines compatible coherences.

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

The Boolean derivations reveal an application at the concrete `And.True.False.Extra` source. Its body receives `True.False.Extra` and returns `False.Extra`. Intermediate descriptions justify applications; they do not accumulate as extra operands. The runtime explores alternative events and shares equivalent configurations.

## Run

Install **Bazel 9.2.0**. No separately installed Rust, Cargo, or native compiler is required.

```sh
bazel run -c opt //system:command -- run "$PWD/example/conjunction.wave"
bazel run -c opt //system:command -- run "$PWD/example/capture.wave" --json
bazel test //...
bazel run //:format -- --check
```

Native source uses only the original concepts, dots, commas, brackets, parentheses, and whitespace. `Seed.A [Seed] [A] B` produces locally executable code. No sigils, constructors, arrow operators, semicolon terminators, braces, or keywords are added. See the [frontend contract](document/syntax.md), [generalization](document/generalization.md), and [examples](example).

The CLI also accepts JSON for the same positive rule model. There are no negative premises or built-in logical concepts. `Not`, `True`, and `False` acquire behavior only through user rules.

The ground runtime and original-syntax lowering are implemented. Execution limits suspend exploration. The report distinguishes a closed finite graph from queued or deferred work. Use `--steps`, `--states`, `--coherences`, `--cells`, `--frames`, and `--records` to adjust limits. `--workers 4` enables optional Rayon workers; the default is one. The record threshold is checked between coordinator batches and is not a byte cap. Embedded callers can resume the same `runtime::Runtime` through `run`. JSON reports include configurations, events, witness mappings, consuming footprints, read dependencies, and support status; they are inspection reports, not restorable checkpoints.

## Core

- Crate-backed parsing and diagnostics lower native source into typed program values.
- Interned atom names, recursive immutable rule values, shared configurations, exact canonical identity, and indexed dependency propagation implement the graph.
- Resumable matching gates retain compatible assignments and suppress duplicate arrivals. Cached bindings reach subscribed views incrementally through one agenda.
- Relative occurrence maps preserve inherited sharing, independent results, lexical captures, and return continuations.
- Indexed positive evidence closure requires every premise and preserves established conclusions. Cycles cannot establish themselves without evidence.

The named crate root [photonic.rs](system/photonic.rs) exposes focused modules directly, without `lib.rs` or re-exports. `lowering` handles executable syntax; `program` interns it; `state`, `refinement`, `canonical`, `matching`, `search`, `flow`, `support`, and `runtime` implement evaluation; `executor` supplies ordered parallel work; `snapshot` supplies inspectable reports. The structural `parser` API and generic `particle` / `rule` multiset kernel remain independent tools. Structural `parse` success alone does not establish executability.

Rust conformance checks cover all 20 programs exported from the JavaScript reference: full canonical configuration, application-edge, and support comparisons for 19 closed cases, plus suspension for one growing case. The Rust tests also cover matching/canonicalization suspension, graph refinement, fair progress, positive evidence closure, and identical full reports across 1/2/4 workers and different pause sizes. Positive support is compared against a simple fixed-point oracle over 14,425 clause programs.

The [runtime chapter](index.html#runtime) explains the implementation and lets you follow native Rust events. The [reference laboratory](document/plan.html) retains the independent ground evaluator. The [semantic contract](document/semantics.md), [implementation plan](document/plan.md), and [terminology](document/terminology.md) describe the accepted model. Earlier HTML laboratories are marked historical.

Whole-rule production and replacement use the same matching, projection, activation, and support machinery as ordinary concepts. Arbitrary structural extraction, binding encodings, induction, and restorable checkpoints remain research work. The former variable/constructor extension has been removed. Graph refinement reduces symmetric work, but exact enumeration can still take factorial time. State sharing prevents repeated equivalent configurations, not genuine fresh-state growth. See [performance](document/performance.md) for the reproducible benchmark and its limits.

## Build

Bazel downloads Rust 1.98.1, hermetic LLVM, platform SDKs, and crates. Initial fetching needs network access; compilation runs without it. Cargo describes dependencies; Bazel owns compilation and testing. Both lockfiles are checked in, and ordinary commands reject stale Bazel resolution data.

```sh
bazel build -c opt //...
bazel test -c opt //...
bazel test //tool:check
bazel build --config=format //...
bazel build --config=lint //...
bazel run -c opt //system:benchmark
```

To update dependencies, run `bazel run //:update --config=refresh`, then `bazel mod deps --config=refresh`. Review both lockfiles and repeat the checks above. Developer commands use the downloaded Rust tools. Bazel creates no convenience symlinks in the checkout; use `bazel info bazel-bin` to locate outputs.

The toolchain targets ARM64 and x86-64 macOS, GNU Linux, and GNULLVM Windows. [CI](.github/workflows/verify.yml) builds and tests optimized binaries on all six native platforms and checks Bazel formatting, Rust formatting, and Clippy in a separate job; see [platform verification](platform/README.md) for host toolchain configuration and matching local commands. Optional remote execution follows Registry; local executor settings belong in ignored `user.bazelrc`.

The crates supply parsing ([pest](https://docs.rs/pest/)), error derivation ([thiserror](https://docs.rs/thiserror/)), diagnostics ([miette](https://docs.rs/miette/)), arguments ([clap](https://docs.rs/clap/)), serialization ([Serde](https://serde.rs/)), stable indexed interning ([IndexMap](https://docs.rs/indexmap/)), and worker pools ([Rayon](https://docs.rs/rayon/)). Photonic owns the rewrite, identity, projection, and support semantics.


## Source layout

- `system/photonic.rs`: language library, with focused modules directly in `system/`.
- `system/command.rs` and `system/command/`: CLI and arguments.
- `system/test/`: integration suite and focused test modules.
- `system/benchmark.rs` and `system/benchmark/`: runtime measurements.
- `mathematics/arithmetic/`: shared digit circuits, numeral encoding, CLI, and tests; `library.rs` is the crate root.
- `mathematics/ternary/`: ordinary-rule digit library and runnable arithmetic examples.

Library modules live beside their crate root. Tests and reporting have separate files. Rust modules are exposed directly, without forwarding re-exports. Bazel targets are `//system:command`, `//system:test`, and `//mathematics/arithmetic:word`; run all tests with `bazel test -c opt //...`.

Build membership is explicit: packages own their source and fixture filegroups, and shared inputs grant visibility only to their consumers. Every maintained Rust executable builds in `//...`; only development tools are manual. The [contribution guide](document/contribution.md) describes package boundaries, naming, and the verification workflow.
