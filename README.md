# Photonic

[Read the interactive webbook](index.html). Photonic programs describe how data changes through rules. Data lives in coherences that can evolve independently or interact when a rule combines them. The native standard library is written in Photonic; Rust supplies the language runtime, build tooling, and verification harness.

## Run and test

Bazel is the only required build tool. It downloads the pinned compiler and dependencies. Rust formatting, Clippy, and Bazel formatting/lint checks run automatically on builds and tests, including dependencies and WebAssembly. Compiler warnings and enabled Clippy lints are errors.

```sh
bazel run -c opt //toolchain/browser:serve
bazel run -c opt //program/composition:pipeline
bazel test -c opt //...
```

Open http://127.0.0.1:8080 after starting the preview. The live examples run the Rust evaluator compiled to WebAssembly; Bazel builds the module and its wasm-bindgen bindings. Recorded diagrams remain available when opening the HTML file directly.

Use `bazel test --config=release //toolchain/browser:check` on ARM64 macOS for the webbook and its Wasm sandbox.

[Continuous verification](document/automation.md) builds and tests Linux x86-64 on Buildkite. See [build organization and performance](document/build.md) for caching, toolchains, and incremental-build measurements.

## Build native programs

```starlark
load("//photonic:defs.bzl", "photonic_binary", "photonic_library", "photonic_test")

photonic_library(
    name = "logic",
    srcs = ["logic.particle"],
    deps = ["//library/boolean:not"],
)

photonic_binary(
    name = "example",
    srcs = ["request.wave", "value.particle"],
    deps = [":logic"],
)

photonic_test(
    name = "negation",
    source = "Invoke.Boolean.Not.True",
    targets = ["Invoke.Boolean.Not.True", "False"],
    preserve = True,
    deps = ["//library/boolean:not"],
)
```

`//library/boolean:not` brings `Invoke` and the Boolean negation table; the [standard library reference](library/README.md) lists every package, request, and answer. Library targets are visible only to the packages that use them, so a new consumer first adds its package to the target's `visibility`. A library contains declarations only; loading them introduces live root rule occurrences. A binary combines the initial configurations from its sources and loads each transitive dependency file once. Both `.particle` and `.wave` use the same grammar; the extensions distinguish reusable definitions/data from executable examples by convention.

A test accepts literal `source`, file `srcs`, and library `deps`. `targets` is a list of complete configurations, including live rules. Every test states `preserve`: when true, each expected target also includes the loaded root rules. The default `match = "all"` requires every target to be reachable; `match = "any"` accepts any one. `expect = "unreachable"` checks non-reachability instead. Unknown never satisfies either expectation. Full exploration shares one execution graph across the targets. Optional `path = True` follows direct paths and can only establish reachability. Reaching all targets does not claim they occur together or exhaust all possible outcomes.

## Runtime values and exact targets

Loading introduces each written rule as a live value in its lexical context. Matching can consume that occurrence; execution reads it. Consumption is local to the selected owner, including when coherences sharing a resource merge. `[] A` starts without initial data; `[()] A` selects an actual coherence. See the [complete occurrence contract and migration evidence](document/occurrence.md).

An exact target includes every surviving root rule. For `A [A] B`, the target `B [A] B` is reachable; `B` alone is not. Targets never inherit rules implicitly. To explicitly construct a target that preserves all written root rules:

```sh
bazel run -c opt //command:photonic -- lower "$PWD/program/natural/result.particle" \
  --context "$PWD/program/natural/addition.wave" > /tmp/photonic-target.json
bazel run -c opt //command:photonic -- prism "$PWD/program/natural/addition.wave" \
  --target /tmp/photonic-target.json --json
```

`bazel run` starts the command in its runfiles directory, so pass absolute paths.

`lower --context` copies the selected source's root rules into the emitted JSON without copying its initial data or modifying rule contents. Use a hand-written complete target when rules are consumed or introduced. Repeat `--context` to explicitly include additional library sources. The arithmetic runner's `--directory` exports `program.wave` and a complete `target.json`.

## Find the implementation

| Directory | Responsibility |
| --- | --- |
| [frontend/](frontend/) | Syntax, parsing, lowering, source types, and frontend conformance tests. |
| [language/](language/) | Execution, Prism verification, and runtime conformance tests. |
| [command/](command/) | Native command interface, diagnostics, and process integration tests. |
| [library/](library/) | The standard library: fourteen Photonic packages with collision-free namespaces. |
| [program/](program/) | Runnable Photonic programs, organized by subject. |
| [theorem/](theorem/) | [Theorems](theorem/README.md) about the standard library, each proved by executing its claim in every case. |
| [arithmetic/](arithmetic/) | Host circuit construction, infix expression encoding, and conformance checks. |
| [benchmark/](benchmark/) | Runtime performance, symmetry, and execution measurements. |
| [photonic/](photonic/) | Bazel rules, source assembly, portable launcher, and Prism test runner. |
| [toolchain/](toolchain/) | Pinned tool execution and build checks. |
| [toolchain/browser/](toolchain/browser/) | WebAssembly adapter, preview server, and browser verification. |
| [platform/](platform/) | Native platform definitions and toolchain patch. |
| [document/](document/) | Contracts, the runtime roadmap, dated runtime records, and webbook assets; see its [index](document/README.md). |
| [code/](code/) | Label-free program representation for the whole grammar, canonical identity, rule analogy, distance, and tree navigation. |
| [machine/](machine/) | Exact exhaustive and parallel execution of flat programs. |
| [translation/](translation/) | Conversion between `code`, runtime source values, and Photonic text. |
| [network/](network/) | Pointer transformer, gradients, optimizer, checkpoints, and function-preserving growth. |
| [gpu/](gpu/) | Metal inference and training for the network; its only unsafe code is the runtime binding. |
| [learning/](learning/) | The [program optimization learner](document/learning.md): edits, objective and goals, planning, self-play, lessons, training, exhaustive and guided solving, and the curriculum. |
| [random/](random/) | Seeded random generation shared by the learning layers. |

Programs belong to a subject: `language/`, `association/`, `composition/`, `natural/`, `binary/`, `decimal/`, `ternary/`, `circuit/`, or `vector/`. Keep a program's expected configurations and regression tests beside its source. A counterexample is a tested outcome, not a separate category of program: `program/natural/preparation.wave` retains the rejected multiplication construction and checks its incorrect result. Generated circuit programs live in `program/circuit/`; their Rust generator lives in `arithmetic/`.

Use `library/` for reusable declarations, one package per type, and `program/` for executable applications. Local declarations that serve one subject stay with that subject. Test-only inputs live in `case/`; recorded demonstrations live in `demo/`. Runtime reference data stays in `language/test/`.

Every runnable `.wave` has a `photonic_binary` target. Discover programs and tests with:

```sh
bazel query 'kind(".*_test rule", //...)'
bazel query 'kind("hermetic_binary rule", //program/...)'
rg --files library program
```

## Learn to optimize programs

The [program optimization learner](document/learning.md) edits a program's rules with a planner guided by a label-free transformer, keeps every example correct under all schedules, and minimizes parallel time and program size under each task's goal. Its network, task pool, and discoveries persist in `~/.cache/photonic/learning`, so every use continues training. `solve` finds the cheapest small program for a task, even one defined only by its tests, and proves it optimal by searching every flat program that could beat it; `improve` and `curriculum` generate new behaviors, solve them and learn from every solution:

```sh
bazel run -c opt //learning:learning -- train --duration 3600
bazel run -c opt //learning:learning -- optimize --program "$PWD/rules.particle" --input "$PWD/input.wave"
bazel run -c opt //learning:learning -- solve --task boolean.and
bazel run -c opt //learning:learning -- curriculum --duration 7200
```

## Prove theorems

A [theorem](theorem/README.md) lists every case of a finite domain, checks its claim in each case, and concludes `Theorem` only when every case holds; its proof test checks that the program reaches exactly `Theorem`. Claims are stated as generally as a finite check allows, with every term and proposition written as Photonic structure: laws of every Boolean algebra, of every relation on any domain, of orders on every type built from products, sums and sequences, of every group, lattice and ring, and of arithmetic at every width. The standard library is proved against ground truth: its digit tables are counting with the successor, and its linked arithmetic, run by the library's own rules one step at a time, is correct at every width:

```sh
bazel test -c opt //theorem/...
bazel test -c opt //theorem/sequence:transitive.proof
```

## Read by subject

| Guide | Contents |
| --- | --- |
| [Language](index.html#guide-language) | Syntax, configuration semantics, identity, binding, and terminology. |
| [Native library](index.html#guide-library) | Packages, namespaces, protocols, chains and alphabets, vectors and sorting, atomic fields, evidence, and remaining work. |
| [Arithmetic](index.html#guide-arithmetic) | Number representations, written arguments, algorithms, and host circuit interface. |
| [Verification](index.html#guide-verification) | Prism, exact configurations, and retained counterexamples. |
| [Runtime](index.html#guide-runtime) | Implementation and reproducible performance measurements. |
| [Runtime roadmap](document/roadmap.md) | Current runtime priorities, acceptance gates, and the semantic boundary. |
| [Documentation index](document/README.md) | Contracts, plans, dated runtime records, and archived proposals. |
| [Build and development](index.html#guide-build) | Bazel interfaces, contributions, and platform verification. |

The webbook is the main guide; the [standard library reference](library/README.md), the [occurrence contract](document/occurrence.md), and the other contracts in the [documentation index](document/README.md) are maintained with it. The webbook's technical reference includes the full contracts, written arithmetic arguments, current limitations, and reproducible benchmark evidence. Prism checks concrete reachability; it does not provide universal mathematical certificates. Native linked ternary expressions support arbitrary finite widths with sufficient execution budgets.
