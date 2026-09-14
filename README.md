# Photonic

[Read the interactive webbook](index.html). Photonic programs describe how data changes through rules. Data lives in coherences that can evolve independently or interact when a rule joins them. The native standard library is written in Photonic; Rust supplies the language runtime, build tooling, and verification harness.

## Run and test

Bazel is the only required build tool. It downloads the pinned compiler and dependencies.

```sh
bazel run -c opt //toolchain/browser:serve
bazel run -c opt //program/composition:pipeline
bazel test -c opt //...
bazel test //toolchain:check
bazel build --config=format //...
bazel build --config=lint //...
```

Open http://127.0.0.1:8080 after starting the preview. The live examples run the Rust evaluator compiled to WebAssembly; Bazel builds the module and its wasm-bindgen bindings. Recorded diagrams remain available when opening the HTML file directly.

Use `bazel test //toolchain/browser:check` on ARM64 macOS for the webbook and its Wasm sandbox.

## Build native programs

```starlark
load("//photonic:defs.bzl", "photonic_binary", "photonic_library", "photonic_test")

photonic_library(
    name = "logic",
    srcs = ["logic.particle"],
)

photonic_binary(
    name = "example",
    srcs = ["request.wave", "value.particle"],
    deps = [":logic"],
)

photonic_test(
    name = "negation",
    source = "Invoke.Not.True",
    targets = ["Invoke.Not.True", "False"],
    deps = [":logic"],
)
```

The example `logic.particle` must supply the application and Boolean rules. A library contains declarations only. A binary combines the initial configurations from its sources and loads each transitive dependency file once. Both `.particle` and `.wave` use the same grammar; the extensions distinguish reusable definitions/data from executable examples by convention.

A test accepts literal `source`, file `srcs`, and library `deps`. `targets` is a list of complete configurations. The default `match = "all"` requires every target to be reachable; `match = "any"` accepts any one. `expect = "unreachable"` checks non-reachability instead. Unknown never satisfies either expectation. Full exploration shares one execution graph across the targets. Optional `path = True` follows direct paths and can only establish reachability. Reaching all targets does not claim they occur together or exhaust all possible outcomes.

## Find the implementation

| Directory | Responsibility |
| --- | --- |
| [language/](language/) | Parsing, lowering, execution, Prism verification, and runtime conformance tests. |
| [command/](command/) | Native command interface, diagnostics, and process integration tests. |
| [library/](library/) | Reusable Photonic declarations, organized by protocol. |
| [program/](program/) | Runnable Photonic programs, organized by subject. |
| [arithmetic/](arithmetic/) | Host circuit construction and conformance checks. |
| [benchmark/](benchmark/) | Runtime performance, symmetry, and execution measurements. |
| [photonic/](photonic/) | Bazel rules, source assembly, portable launcher, and Prism test runner. |
| [toolchain/](toolchain/) | Pinned tool execution and build checks. |
| [toolchain/browser/](toolchain/browser/) | WebAssembly adapter, preview server, and browser verification. |
| [platform/](platform/) | Native platform definitions and toolchain patch. |
| [document/](document/) | Webbook assets and benchmark evidence. |

Programs belong to a subject: `language/`, `association/`, `composition/`, `natural/`, `binary/`, `decimal/`, `ternary/`, or `circuit/`. Keep a program's expected configurations and regression tests beside its source. A counterexample is a tested outcome, not a separate category of program: `program/natural/preparation.wave` retains the rejected multiplication construction and checks its incorrect result. Generated circuit programs live in `program/circuit/`; their Rust generator lives in `arithmetic/`.

Use `library/` for reusable protocols and `program/` for executable applications. Local declarations that serve one subject stay with that subject. Test-only inputs live in `case/`; recorded demonstrations live in `demo/`. Runtime reference data stays in `language/test/`.

Every runnable `.wave` has a `photonic_binary` target. Discover programs and tests with:

```sh
bazel query 'kind(".*_test rule", //...)'
bazel query 'kind("rust_binary rule", //program/...)'
rg --files library program
```

## Read by subject

| Guide | Contents |
| --- | --- |
| [Language](index.html#guide-language) | Syntax, configuration semantics, identity, binding, and terminology. |
| [Native library](index.html#guide-library) | Protocols, atomic fields, dependency layers, evidence, and remaining work. |
| [Arithmetic](index.html#guide-arithmetic) | Number representations, written arguments, algorithms, and host circuit interface. |
| [Verification](index.html#guide-verification) | Prism, exact configurations, and retained counterexamples. |
| [Dynamic code proposal](document/dynamic.md) | Research, contextual rule construction, capture, recursive growth, and implementation gates. |
| [Runtime](index.html#guide-runtime) | Implementation and reproducible performance measurements. |
| [Build and development](index.html#guide-build) | Bazel interfaces, contributions, and platform verification. |

The webbook is the single maintained guide. Its technical reference includes the full contracts, written arithmetic arguments, current limitations, and reproducible benchmark evidence. Prism checks concrete reachability; it does not provide universal mathematical certificates. Native linked ternary expressions support arbitrary finite widths with sufficient execution budgets.
