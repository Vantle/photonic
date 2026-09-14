# Build and development

[Webbook](../index.html) · [Repository guide](../README.md)

- [Photonic build rules](#rule)
- [Contributing](#contribution)
- [Platform verification](#platform)

<a id="rule"></a>

## Native build rules

```starlark
load("//photonic:defs.bzl", "photonic_binary", "photonic_library", "photonic_test")

photonic_library(
    name = "logic",
    srcs = ["application.particle", "boolean.particle"],
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

List files explicitly. Bazel visibility controls consumers. `photonic_library` validates declaration-only files and exports their transitive source dependencies. Including a library introduces no initial data. Empty source lists support dependency aggregation when that is useful; avoid a target that merely renames another module. Diamond dependencies include a shared file once. Different files with identical rules are distinct declarations.

`photonic_binary` combines initial configurations and declarations from N independently parsed `srcs`, then loads the library closure. A file cannot be both a source and a dependency. The assembler emits existing Photonic JSON without evaluating, specializing, or generating library behavior. Syntax errors identify their source file. The native launcher resolves Bazel runfiles without a workspace-relative path, shell, or installed interpreter. The default operation is `run`; explicit `obsidian` and runtime arguments retain CLI semantics. The generated `<name>.program` target exposes the assembled artifact with the binary's visibility.

```sh
bazel run -c opt //example/library:pipeline -- --json
bazel run -c opt //mathematics/ternary:position -- obsidian \
  --target "$PWD/mathematics/ternary/position.particle" --path --cells 128
bazel test -c opt //mathematics/ternary:position.check
```

<a id="test"></a>

## Obsidian tests

`photonic_test` accepts:

| Attribute | Meaning |
| --- | --- |
| `source` | Literal Photonic code containing initial data and/or declarations; use `""` with file-only tests. |
| `srcs` | Optional native source files, combined with `source`. |
| `deps` | Optional declaration libraries. |
| `targets` | Nonempty list of complete configurations in literal Photonic syntax; no target declarations. |
| `match` | `"all"` by default; `"any"` accepts one matching target. |
| `expect` | `"reached"` by default, or `"unreachable"`. |
| `path` | `False` by default; `True` requests direct-path reachability witnesses. |
| `steps`, `states`, `cells`, `frames`, `coherences`, `records` | Explicit nonnegative exploration budgets. |
| `size`, `tags`, `visibility` | Standard Bazel test controls. |

```starlark
photonic_test(
    name = "choice",
    source = "A [A] B [A] C",
    targets = ["B", "C"],
)

photonic_test(
    name = "alternative",
    source = "A [A] B",
    targets = ["B", "C"],
    match = "any",
)

photonic_test(
    name = "exclusion",
    source = "A [A] B",
    targets = ["C", "D"],
    expect = "unreachable",
)
```

All targets are parsed and validated before execution. Full exploration runs once, then queries that shared graph for every target. `all` requires each query to satisfy `expect`; `any` stops at the first satisfying query. Unknown satisfies neither expectation. A closed graph can establish unreachability; a direct path cannot, so `path = True` with `expect = "unreachable"` is rejected during analysis. In direct-path mode each target receives its own path search and budget.

Each entry describes a whole configuration: `"A, B"` requires two jointly present coherences; `["A", "B"]` asks separate queries. `all` does not require the targets to coexist or share an execution history, nor does it assert that no unlisted states are reachable. Use explicit unreachable tests for forbidden results.

Bazel test success/failure reflects the expectation, including a failure on inconclusive checks. This differs from the inspection CLI, which reports an Obsidian outcome as data. Full-exploration tests save one `execution.json` graph and small `0.json`, `1.json`, and subsequent target verdicts in Bazel's undeclared test outputs. Direct-path tests retain a separate complete path report for each attempted target. A failure log identifies the target and whether its outcome was Reached, Unreachable, or Unknown. Assembly, parsing, missing runfiles, and invalid configuration errors fail the process.

Build tests cover source assembly, dependency diamonds, execution from another directory, manifest-only runfiles, argument handling, target matching, invalid targets, and budget exhaustion.

<a id="contribution"></a>

## Contributing

Bazel is the build entry point. The version in `.bazelversion` selects Bazel; `MODULE.bazel`, `MODULE.bazel.lock`, and `Cargo.lock` select the downloaded tools and dependencies. A local Rust, Cargo, C compiler, or formatter installation is unnecessary.

<a id="contribution-structure"></a>

### Structure

Use short, familiar, singular names. Give a module one responsibility and let its directory provide context. Keep implementation details private. Introduce a public type or module when another layer needs a stable contract; avoid forwarding aliases and compatibility wrappers.

The runtime library owns parsing, lowering, execution, and inspectable reports. The command layer owns arguments, filesystem access, diagnostics, and presentation. Arithmetic generates ordinary Photonic source and target encodings; the runtime executes those programs through its general rewrite machinery. Neither layer depends on the other layer's command interface. Documentation and fixtures describe and exercise these contracts.

Keep a Rust crate root beside its modules. Split a growing module when its responsibilities separate, using a directory for its private implementation. For example, `system/runtime/report.rs` constructs reports without mixing presentation into scheduling. Prefer this kind of boundary over dividing files by line count.

<a id="contribution-build-ownership"></a>

### Build ownership

Every build input is listed explicitly. Do not use `glob`, recursive source discovery, or directory-wide exports in build rules. Group files by their role, such as `source`, `fixture`, or `build`; avoid groups that merely rename one file without providing a boundary.

Packages default to private visibility. A shared filegroup grants access only to its consuming package. The runtime library is public; the arithmetic library is available within mathematics. Platform definitions are public configuration inputs, while the toolchain patch is visible only to the root module.

Keep maintained executables in ordinary `//...` builds. Reserve `manual` for development tools and checks that need a specific platform. The browser check runs explicitly in its ARM64 macOS CI job. The native matrix compiles and executes on ARM64 and x86-64 macOS, Linux, and Windows. It also runs the formatter and checks the dependency command on every platform. These commands use the shared Rust runner, so they require no shell launcher. See [platform verification](#platform) for native and cross-compilation commands.

Declare files read during compilation in `compile_data`. Declare executables and files read at runtime in `data`. Pass executable locations with `rlocationpath` and resolve them through the standard runfiles library; paths relative to the working directory do not provide a portable executable location. The shared runner resolves file arguments before `--`, forwards subsequent arguments literally, and propagates runfile lookup to child tools. `//tool:runner` covers nested manifest lookup and exit codes without relying on a physical symlink tree.

Each package exposes its `BUILD.bazel` through a private-to-root `build` filegroup. Add that group to the root `build` list when introducing a package. This list supplies the Bazel formatting check in `tool/`. Development-only dependencies stay in that package so loading the library does not require them. Rust formatting and linting follow the build graph, so an ordinary Rust target participates automatically.

<a id="contribution-verification"></a>

### Verification

Run these commands from the repository root:

```sh
bazel build -c opt //...
bazel test -c opt //...
bazel test //tool:check
bazel build --config=format //...
bazel build --config=lint //...
```

Clippy warnings fail verification. Fix the underlying issue instead of suppressing a diagnostic globally. Preserve independent test oracles: native runtime reports are compared with reference fixtures, support propagation is compared with a fixed-point implementation, and arithmetic results are checked through runtime execution. `//tool:test` executes the independent JavaScript reference and compares regenerated fixtures to the Rust oracle. `//tool:browser` checks theme persistence, all four recorded arithmetic results, event navigation, reference exploration, binding deduplication, and asset loading in pinned headless Chrome. Run that target on ARM64 macOS; no host Node, browser, package manager, or shell launcher is needed.

Format Rust with the package's formatter:

```sh
bazel run //:format
bazel run //mathematics/arithmetic:format
bazel run //mathematics/binary:format
```

Format a changed Bazel file with the pinned tool:

```sh
bazel run @buildifier_prebuilt//:buildifier -- system/BUILD.bazel
```

When adding a crate root, include it in the appropriate formatter entry point. The formatting aspect checks declared Rust source files independently of those convenience commands.

Add tests at stable interface boundaries. Cover failure, suspension, resumption, and deterministic behavior where they are part of the contract. Keep arithmetic expectations independent of circuit generation. The saved add, multiply, subtract, and divide programs are fixtures with explicit expected results.

<a id="contribution-dependencies-and-performance"></a>

### Dependencies and performance

Update dependencies explicitly:

```sh
bazel run //:update --config=refresh
bazel mod deps --config=refresh
```

Review both lockfiles and repeat verification. Ordinary builds reject stale resolution data; refresh mode belongs to dependency maintenance. Keep local executor settings in ignored `user.bazelrc` and shared behavior in `.bazelrc`.

Use optimized builds for measurements, preserve the input and limits, and compare observable results before accepting an optimization. The [performance guide](runtime.md#performance) documents the runtime benchmark and [arithmetic guide](arithmetic.md#word) documents digit circuits. Keep measured results separate from algorithmic assumptions.

<a id="platform"></a>

## Platform verification

Bazel target platforms specify both architecture and operating system. Linux uses the hermetic glibc 2.28 baseline. Windows targets specify GNULLVM and MSVCRT explicitly. The `windows` host platform preserves the detected architecture and selects the same ABI for compiler tools, build scripts, and procedural macros.

[rust.patch](../platform/rust.patch) selects the upstream GNULLVM Rust host distributions in the pinned `rules_rs` dependency. Its default MSVC compiler cannot load GNULLVM procedural macros, while the hermetic LLVM C/C++ toolchain supports the MinGW ABI. Selecting matching compiler distributions keeps the entire Windows build hermetic. The toolchain uses upstream Rust archives because the redistributed archive set omits these Windows host distributions. Compiler archive hashes are recorded in `MODULE.bazel.lock`.

The patch also extracts the compiler distribution beside Windows Cargo and declares `libunwind.dll` explicitly as runtime data. The GNULLVM Cargo archive omits `libunwind.dll`, which it needs even for dependency metadata; no runner-installed DLL or compiler is required.

| Target | Native CI runner |
| --- | --- |
| `//platform:aarch64-apple-darwin` | `macos-15` |
| `//platform:x86_64-apple-darwin` | `macos-15-intel` |
| `//platform:aarch64-unknown-linux-gnu` | `ubuntu-24.04-arm` |
| `//platform:x86_64-unknown-linux-gnu` | `ubuntu-24.04` |
| `//platform:aarch64-pc-windows-gnullvm` | `windows-11-arm` |
| `//platform:x86_64-pc-windows-gnullvm` | `windows-2025` |

The [verification workflow](../.github/workflows/verify.yml) builds and tests optimized binaries on each configured native runner. A separate Linux job checks explicit Bazel formatting inputs and applies Rust formatting and Clippy to every maintained Rust target. Clippy warnings fail the check. Bazel 9.2.0 comes from `.bazelversion`; the workflow pins Bazelisk 1.28.1, checkout 6.0.2, and setup-bazel 0.19.0. Actions use immutable commit references. Cargo is not a workflow command. Runner labels identify operating-system families and architectures, but hosted images receive updates. The labels come from GitHub's [runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners); Bazel bootstrap and caching follow [setup-bazel](https://github.com/bazel-contrib/setup-bazel/tree/c5acdfb288317d0b5c0bbd7a396a3dc868bb0f86).

Native jobs explicitly set both `--host_platform` and `--platforms` to the matching platform. This gives Bazel's test execution platform the same constraints as the target, including the Linux glibc baseline. Set the host platform only to one matching the machine's architecture and operating system. Cross-builds change only `--platforms` and retain the detected host.

Bazelisk and repository downloads are cached on every runner. Windows CI disables the expanded repository contents cache with `--repo_contents_cache=` while retaining downloaded archives. The expanded directory can change during compression, causing cache archival to fail after several minutes. Native jobs stop Bazel before saving caches. The runtime and arithmetic suites have Bazel's medium test size, allowing five minutes per suite.

From the repository root, cross-compile the CLI without attempting to execute the target binary:

```sh
bazel build //system:command --platforms=//platform:x86_64-unknown-linux-gnu
bazel build //system:command --platforms=//platform:x86_64-pc-windows-gnullvm
```

To verify on a matching native host:

```sh
bazel build -c opt //... --host_platform=//platform:aarch64-apple-darwin --platforms=//platform:aarch64-apple-darwin
bazel test -c opt //... --host_platform=//platform:aarch64-apple-darwin --platforms=//platform:aarch64-apple-darwin
bazel test //tool:check
bazel build --config=format //...
bazel build --config=lint //...
```

Replace the example target with the matching platform from the table. Cross-compilation alone does not establish native execution; the workflow runs the tests on each matching host.

CLI integration tests declare executables as data and resolve `rlocationpath` values with the standard Rust runfiles library. This supports both directory runfiles and Windows manifests.

<a id="api"></a>

## API commitments

The project is still version 0.0.0. The following supported interfaces have explicit regression coverage; that is a compatibility commitment for deliberate changes, not a claim that every public implementation detail is permanently frozen.

| Surface | Contract |
| --- | --- |
| Native source | Existing positive-rule grammar, orderless multiplicity, explicit coherence association, and captured whole-rule values. No primitive arithmetic or hidden argument binder. |
| `photonic_library` | Declaration-only source closure; shared files included once; no implicit initial configuration. |
| `photonic_binary` | Explicit source files and library dependencies; runfiles-based execution. |
| `photonic_test` | `source`, `srcs`, `deps`, `targets`, `match`, `expect`, and explicit nonnegative budgets. Unknown satisfies no expectation. |
| Obsidian | `Search::new`, `run`, `parallel`, `target`, `verdict`, and `report`. Retargeting preserves completed work; invalid targets leave the existing query intact. |
| Browser | Version 1 JSON request/response through generated `execute(request: string): string` bindings. Unknown request fields and unsupported versions are rejected. |

`Search::verdict()` returns only outcome and witness. It looks up the canonical target in the existing state index and reads cached support without formatting the execution graph. Hashing still depends on target size. `report()` deliberately materializes the graph for inspection; callers should avoid it in query loops. Reports contain run-local state and event identifiers, not persistent object handles or a resumable checkpoint format. Internal event ordering, work counts, generated artifact names, and pretty-printed diagnostics are not compatibility promises.

Changes to observable semantics require conformance tests and an explicit migration note here. Breaking browser message changes require a new version; additive inspection-report fields may be ignored by consumers. Do not promote a generic collection or function-binding contract merely because a small example succeeds. The native library's finite carriers and proof boundaries remain documented in the [library guide](library.md).

<a id="browser"></a>

## Live Rust in the webbook

```sh
bazel run -c opt //browser:serve
bazel test -c opt //browser:test
```

Open http://127.0.0.1:8080 for the live webbook. The local preview serves checkout assets and generated bindings; it is a development server bound to loopback. The HTML file still opens directly for offline reading and recorded diagrams.

`//browser:module` uses upstream `rust_wasm_bindgen` with `target = "web"`, supplied through the pinned `rules_rs` module extension. It builds the WebAssembly engine, JavaScript loader, and TypeScript declarations hermetically. The Rust crate and CLI come from the same pinned toolchain dependency set. There is no wasm-pack, npm install, system Rust compiler, handwritten allocation ABI, or custom cross-compilation transition.

The [upstream rule reference](https://bazelbuild.github.io/rules_rust/rust_wasm_bindgen.html) describes the generated outputs and toolchain selection. The [rules_rs extension](https://github.com/hermeticbuild/rules_rs/blob/main/rs/rules_rust_wasm_bindgen.bzl) supplies the matching hermetic tools. Upgrading either is a dependency change: review the lockfile, regenerate bindings, and rerun native/WebAssembly equivalence tests.

The worker accepts `{ "version": 1, "source": "A [A] B", "targets": ["B"] }`. Responses contain `version`, a complete `execution` snapshot, and a `verdict` array in target order; validation failures contain `version` and an `error` object with `code` and human-readable `message`. Stable engine error codes are `request`, `version`, `size`, `source`, and `target`; the worker can also report `worker` for loading or execution failures. Message wording is diagnostic text, not an API identifier. Empty target lists request a graph without queries. Requests are limited to 32 KiB and 16 targets. Execution uses 20,000 work steps, 128 states and cells, 16 frames and coherences, and 100,000 records. These are exploration thresholds, not a byte-perfect memory cap. The UI terminates the worker after five seconds or when Stop is pressed. Cancellation establishes no verdict.

Native/WebAssembly tests compare complete reports for ordinary rewriting, source inference, local generated code, coherence reunion, and suspended growth. Browser tests cover graph selection, live positive and negative queries, reset, and cancellation. The browser runs the serial Rust evaluator; cheap Photonic coherences remain language-level parallel structure, not a claim of browser thread-level speedup.
