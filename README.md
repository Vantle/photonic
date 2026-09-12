# Molten

Computational expression over hypergraphs, with a native Rust frontend and runtime. The hermetic build follows [Vantle Registry](https://github.com/Vantle/registry).

A **rule** describes computation, including abstraction derivations; an **event** records one application. Types are computation in the same iterative model. A **coherence** is an independently evolving parallel context. **Divergence** creates several coherences; **decoherence** combines compatible coherences.

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

The Boolean derivations reveal an application at the concrete `And.True.False.Extra` source. Its body receives `True.False.Extra` and returns `False.Extra`. Intermediate descriptions justify applications; they do not accumulate as extra operands. The runtime explores alternative events and shares equivalent configurations.

## Run

Install **Bazel 9.2.0**. No separately installed Rust, Cargo, or native compiler is required.

```sh
bazel run -c opt //system/molten/command -- run "$PWD/example/conjunction.lava"
bazel run -c opt //system/molten/command -- run "$PWD/example/capture.lava" --json
bazel test //...
bazel run //:format -- --check
```

Native source supports joint inputs, multiple outputs, nested bodies, whole rule values, and absence premises. For example, `Seed.A; [Seed] -> @([A] -> B);` produces locally executable code. `run` also accepts the structured JSON program format; `--format molten|json` overrides extension detection. See the [frontend contract](document/syntax.md) and [examples](example).

Execution limits suspend exploration. The report distinguishes a closed finite graph from queued or deferred work. Use `--steps`, `--states`, `--coherences`, `--cells`, and `--frames` to adjust limits. Embedded callers can resume the same `runtime::Runtime` through `run`. JSON reports include configurations, events, witness mappings, consuming footprints, read dependencies, and support status; they are inspection reports, not restorable checkpoints.

## Core

- Crate-backed parsing and diagnostics lower native source into typed program values.
- Interned symbols and rule code, immutable shared configurations, exact canonical identity, and indexed dependency propagation implement the graph.
- Matching gates retain compatible assignments and suppress duplicate arrivals. Cached bindings feed concrete-source inference and application through one agenda.
- Relative occurrence maps preserve inherited sharing, independent results, lexical captures, and return continuations.
- Well-founded support keeps negative cycles conditional and records defeated assumptions without rejecting logical programs.

The named crate root [molten.rs](system/molten/molten.rs) exposes focused modules directly, without `lib.rs` or re-exports. `lowering` handles executable syntax; `program` interns it; `state`, `matching`, `flow`, `support`, and `runtime` implement evaluation; `snapshot` supplies inspectable reports. The structural `parser` API and generic `particle` / `rule` multiset kernel remain independent tools. Structural `parse` success alone does not establish executability.

Rust conformance checks cover all 26 programs exported from the JavaScript reference: full canonical configuration, application-edge, and support comparisons for 24 closed cases, plus suspension for two growing cases. Separate regression tests cover identity permutations, capture sharing, matching gates, budget resumption, native lowering, and the CLI.

The [interactive runtime plan](document/plan.html) runs the reference examples offline. The [semantic contract](document/semantics.md), [implementation plan](document/plan.md), and [terminology](document/terminology.md) describe the accepted model. Earlier HTML laboratories are marked historical.

This milestone executes finite ground rule constructors, including higher-order whole-rule replacement and activation. Arbitrary structural extraction, parallel workers, and restorable checkpoints remain future work. Exact symmetry enumeration can take factorial time; matching candidates and support clauses can grow substantially. State sharing prevents repeated equivalent configurations, not genuine fresh-state growth. See [performance](document/performance.md) for the reproducible benchmark and its limits.

## Build

Bazel downloads Rust 1.98.1, hermetic LLVM, platform SDKs, and crates. Initial fetching needs network access; compilation runs without it. Cargo describes dependencies; Bazel owns compilation and testing. Both lockfiles are checked in, and ordinary commands reject stale Bazel resolution data.

```sh
bazel build //...
bazel run //:format
bazel run //:update --config=refresh
bazel mod deps --config=refresh
bazel test //... --config=refresh
bazel test //...
bazel run -c opt //system/molten/benchmark
```

Review both lockfiles after dependency updates. Developer commands use the downloaded Rust tools. Bazel creates no convenience symlinks in the checkout; use `bazel info bazel-bin` to locate outputs.

The toolchain targets ARM64 and x86-64 macOS, GNU Linux, and GNULLVM Windows. Runtime verification in this checkout used ARM64 macOS; cross-platform configuration alone does not prove native runtime support. Optional remote execution follows Registry; local executor settings belong in ignored `user.bazelrc`.

The crates supply parsing ([pest](https://docs.rs/pest/)), error derivation ([thiserror](https://docs.rs/thiserror/)), diagnostics ([miette](https://docs.rs/miette/)), arguments ([clap](https://docs.rs/clap/)), serialization ([Serde](https://serde.rs/)), and stable indexed interning ([IndexMap](https://docs.rs/indexmap/)). Molten owns the rewrite, identity, projection, and support semantics.
