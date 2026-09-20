# Build organization

Bazel 9.2.0 is the only system build dependency. Rust, LLVM, Node, browser tools, launcher templates, and binding tools are pinned. Build actions remain sandboxed with network access disabled and a strict environment.

## Incremental boundaries

`frontend` owns syntax, parsing, lowering, diagnostics, and source types. Its tests link the frontend library instead of recompiling the runtime. `language` owns execution and Prism; its public frontend modules retain their existing paths. The build-time assembler depends only on `frontend`, so runtime edits cannot invalidate source assembly.

Photonic dependencies use postorder depsets to preserve declaration order and deduplicate diamonds. Each assembly action reads and validates every transitive source itself. It does not wait for redundant library validation artifacts. Library initialization remains forbidden, and binary source/library overlap remains an error.

Every program uses one shared native launcher. Hermetic wrappers use the public API from [HermeticBuild's launcher](https://github.com/hermeticbuild/hermetic-launcher), which patches prebuilt executable templates instead of compiling Rust for each program. The repository provides `hermetic_binary` and `hermetic_test` in `toolchain/defs.bzl`; these are repository rules, not upstream exports. They declare all executable and data runfiles. `platform/launcher.patch` registers the upstream ARM64 Windows template and finalizer that already ship with the pinned release; upstream omits their Bazel registration by default. A shared argument reader handles long or empty embedded arguments through a JSON file because the pinned upstream template has nine usable argument slots and rejects empty embedded arguments. Additional process arguments, Unicode, spaces, exit codes, and manifest-only execution have regression coverage.

The WebAssembly adapter shares the main Rust dependency set. Its `wasm-bindgen` crate and command tool both use version 0.2.105. The public binding toolchain selects checksum-pinned [official releases](https://github.com/wasm-bindgen/wasm-bindgen/releases/tag/0.2.105) on ARM64/x86-64 macOS and Linux. Linux uses static musl tools. Windows retains the existing Rust source toolchain. Update the crate pin, release URLs, checksums, and source fallback together when upgrading binding versions.

## Configuration

Build-time helpers use `fastbuild`, independently of the target compilation mode. This reduced the sampled clean build from 33.270 to 28.994 seconds while leaving optimized runtime builds optimized.

The default target compilation mode remains `fastbuild`. Use `--config=release` for optimized execution; it is equivalent to `-c opt`. Keep the mode consistent across build, test, formatting, and lint commands to avoid discarding Bazel's analysis cache:

```sh
bazel build --config=release //...
bazel test --config=release //...
bazel test --config=release //toolchain/browser:check
```

The default `//toolchain:check.bzl%check` aspect composes the pinned Rust formatting and Clippy aspects and propagates their outputs through dependencies, launchers, and WebAssembly transitions. Both `bazel build` and `bazel test` request its `check` output group in addition to normal outputs. No lint configuration flag is needed. `//toolchain:check` checks Bazel formatting and lint warnings as part of `bazel test //...`.

The lint policy lives in `Cargo.toml`. The documented [rules_rs Cargo lint integration](https://github.com/hermeticbuild/rules_rs#cargo-lint-configuration) produces its Bazel configuration, exposed as `//:lint`. Compilation rules and toolchains come from `rules_rs`; its public compatibility repository supplies the formatting and Clippy aspects that it has not wrapped. No independent `rules_rust` dependency or private API is used.

Every Rust target sets `lint_config = "//:lint"`; the aspect rejects missing configuration. This shared policy denies compiler warnings, Rust 2018 idiom violations, unused lifetimes, standard Clippy lints, and selected checks for redundant clones, copied values, explicit iterator loops, option handling, early returns, mutable references, nested patterns, `Self`, and unfinished debugging code. Following [Clippy's guidance](https://doc.rust-lang.org/clippy/lints.html), pedantic and restriction groups are not enabled wholesale. Add individual rules when they improve this codebase without routine exemptions. Fix diagnostics instead of adding `allow`, `expect`, or skip tags; an exception requires a concrete explanation of why a code fix is impossible. No exemptions are currently required.

Run `bazel run //:format` to format Rust. The lint policy applies to repository code; dependency repositories keep their upstream policy.

The browser check runs on ARM64 macOS. The CI matrix also runs native tests on both CPU architectures for macOS, Linux, and Windows.

[Bazel’s disk action cache](https://bazel.build/remote/caching#disk-cache) lives at `~/.cache/photonic/bazel`. Idle garbage collection evicts entries older than 14 days and limits storage to 20 GB; this is an idle cleanup policy, not a hard instantaneous quota. Input-change checking protects cache uploads. CI restores and saves an action cache in addition to repository and Bazelisk caches; pull requests only restore shared caches. The setup action chooses its platform-specific cache location through the home bazelrc.

Bazel already retains its analysis graph, caches test results, schedules parallel actions, and reuses sandbox directories. Explicit Rust pipelining was measured at 36.118 seconds, slower than the 33.270-second comparison build, so it remains disabled. Retain measured defaults rather than adding flags solely because they are experimental. Keep the Bazel server running and avoid `bazel clean` during ordinary editing. A disk cache helps recover previous outputs after reverting an edit or switching branches; it does not make a new compiler invocation free.

Put machine-specific overrides and remote service endpoints in ignored `user.bazelrc`. `--config=remote` enables remote-only execution with minimal output downloads and no local fallback; a configured executor is required. No external service is needed for local caching.

## Measurement

Local samples compare baseline `a994edf` with this change on an Apple M5 Max running macOS 26.6.2. Both use optimized builds and the same pinned compiler. These are individual elapsed-time samples, not statistical medians. Clean builds have empty build outputs, disabled disk action caching, and cached repository downloads.

| Work | Before | After |
| --- | ---: | ---: |
| Clean `bazel build -c opt //...` | 50.249 s | 28.994 s |
| Runtime edit, native command only | 2.838 s | 2.817 s |
| Runtime edit, all targets | 7.247 s | 6.117 s |
| Restore previously built full-tree state | 7.109 s | 0.192 s |

The full-tree runtime-edit sample executed 21 compiler actions before and 18 after. The remaining critical path is the runtime test binary, so fresh incremental builds still take seconds even though cache recovery is much faster. Returning to the cached full-tree state executed no compiler actions and reused 18 disk-cache results. Performance changes come from build boundaries, shared tools, and caching; the runtime and frontend evaluation semantics are unchanged.

To reproduce a clean-output profile:

```sh
bazel clean
bazel build -c opt //... --disk_cache= --profile=/tmp/photonic-build.json.gz
bazel analyze-profile /tmp/photonic-build.json.gz
```

For incremental measurements, first build with the desired flags, edit a source file, then rebuild with the same flags and a fresh profile path. Changing cache or compilation settings between samples can invalidate the comparison. Restoring the original source and rebuilding separately measures cache recovery.
