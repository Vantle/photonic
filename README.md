# Photonic

[Read the interactive webbook](index.html). Photonic expresses computation through positive rules over independently evolving coherences. The native standard library is written in Photonic; Rust supplies the language runtime, build tooling, and verification harness.

## Run and test

Bazel is the only required build tool. It downloads the pinned compiler and dependencies.

```sh
bazel run -c opt //browser:serve
bazel run -c opt //example/library:pipeline
bazel test -c opt //...
bazel test //tool:check
bazel build --config=format //...
bazel build --config=lint //...
```

The live preview opens at http://127.0.0.1:8080 and runs the Rust evaluator through hermetic wasm-bindgen bindings. Recorded diagrams remain available when opening the HTML file directly.

Use `bazel test //tool:browser` on ARM64 macOS for the webbook and independent reference laboratory.

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
    source = "Apply.Not.True",
    targets = ["Apply.Not.True", "False"],
    deps = [":logic"],
)
```

The example `logic.particle` must supply the application and Boolean rules. A library contains declarations only. A binary combines the initial configurations from its sources and loads each transitive dependency file once. Both `.particle` and `.wave` use the same grammar; the extensions distinguish reusable definitions/data from executable examples by convention.

A test accepts literal `source`, file `srcs`, and library `deps`. `targets` is a list of complete configurations. The default `match = "all"` requires every target to be reachable; `match = "any"` accepts any one. `expect = "unreachable"` checks non-reachability instead. Unknown never satisfies either expectation. Full exploration shares one execution graph across the targets. Optional `path = True` follows direct paths and can only establish reachability. Reaching all targets does not claim they occur together or exhaust all possible outcomes.

## Find the implementation

| Directory | Responsibility |
| --- | --- |
| [library/](library/) | Native reusable Photonic declarations; one module per concern. |
| [example/library/](example/library/) | Composed standard-library applications. |
| [example/](example/) | Language examples, association cases in `group/`, rejected constructions in `counterexample/`. |
| [mathematics/natural/](mathematics/natural/) | Unary mathematical constructions and retained proof fixtures. |
| [mathematics/ternary/](mathematics/ternary/) | Native trit applications, indexed numerals, streams, and sparse powers. |
| [mathematics/binary/](mathematics/binary/), [decimal/](mathematics/decimal/) | Sparse numeral examples. |
| [mathematics/arithmetic/](mathematics/arithmetic/) | Host circuit construction and conformance checks; generated Photonic examples live in `fixture/ternary/`. |
| [browser/](browser/) | WebAssembly bindings, live preview, and native/Wasm conformance tests. |
| [photonic/](photonic/) | Bazel rules, source assembly, portable launcher, and Obsidian test runner. |
| [system/](system/) | Frontend, runtime, Obsidian, command, benchmarks, and conformance tests. |
| [document/](document/) | Canonical guides and webbook assets. |

Every runnable `.wave` has a `photonic_binary` target. Discover programs and tests with:

```sh
bazel query 'kind(".*_test rule", //...)'
bazel query 'kind("rust_binary rule", //...)'
rg --files library example mathematics photonic
```

## Read by subject

| Guide | Contents |
| --- | --- |
| [Language](document/language.md) | Syntax, configuration semantics, identity, binding, and terminology. |
| [Native library](document/library.md) | Protocols, atomic fields, dependency layers, evidence, and remaining work. |
| [Arithmetic](document/arithmetic.md) | Number representations, written arguments, algorithms, and host circuit interface. |
| [Verification](document/verification.md) | Obsidian, exact configurations, and retained counterexamples. |
| [Runtime](document/runtime.md) | Implementation and reproducible performance measurements. |
| [Build and development](document/build.md) | Bazel interfaces, contributions, and platform verification. |

The [reference laboratory](document/reference.html) remains an independent executable model used for conformance. The [physical research report](document/research.html) records the physical motivation and sources. Neither is a replacement implementation of the native library.

Current native collection protocols cover finite pairs; general recursive argument binding, arbitrary repeat counts, and arbitrary-width native word arithmetic remain unfinished. Written arguments and finite reachability checks are distinguished from universal machine-checked proofs throughout the guides.
