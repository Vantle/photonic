# Contributing

Bazel is the build entry point. The version in `.bazelversion` selects Bazel; `MODULE.bazel`, `MODULE.bazel.lock`, and `Cargo.lock` select the downloaded tools and dependencies. A local Rust, Cargo, C compiler, or formatter installation is unnecessary.

## Structure

Use short, familiar, singular names. Give a module one responsibility and let its directory provide context. Keep implementation details private. Introduce a public type or module when another layer needs a stable contract; avoid forwarding aliases and compatibility wrappers.

The runtime library owns parsing, lowering, execution, and inspectable reports. The command layer owns arguments, filesystem access, diagnostics, and presentation. Arithmetic generates ordinary Photonic source and target encodings; the runtime executes those programs through its general rewrite machinery. Neither layer depends on the other layer's command interface. Documentation and fixtures describe and exercise these contracts.

Keep a Rust crate root beside its modules. Split a growing module when its responsibilities separate, using a directory for its private implementation. For example, `system/runtime/report.rs` constructs reports without mixing presentation into scheduling. Prefer this kind of boundary over dividing files by line count.

## Build ownership

Every build input is listed explicitly. Do not use `glob`, recursive source discovery, or directory-wide exports in build rules. Group files by their role, such as `source`, `fixture`, or `build`; avoid groups that merely rename one file without providing a boundary.

Packages default to private visibility. A shared filegroup grants access only to its consuming package. The runtime library is public; the arithmetic library is available within mathematics. Platform definitions are public configuration inputs, while the toolchain patch is visible only to the root module.

Keep maintained executables in ordinary `//...` builds. Reserve `manual` for development tools, such as dependency updates and formatters. The native matrix compiles and executes on ARM64 and x86-64 macOS, Linux, and Windows. See [platform verification](../platform/README.md) for native and cross-compilation commands.

Declare files read during compilation in `compile_data`. Declare executables and files read at runtime in `data`. Pass executable locations with `rlocationpath` and resolve them through the standard runfiles library; paths relative to the working directory do not provide a portable executable location.

Each package exposes its `BUILD.bazel` through a private-to-root `build` filegroup. Add that group to the root `build` list when introducing a package. This list supplies the Bazel formatting check. Rust formatting and linting follow the build graph, so an ordinary Rust target participates automatically.

## Verification

Run these commands from the repository root:

```sh
bazel build -c opt //...
bazel test -c opt //...
bazel test //:check
bazel build --config=format //...
bazel build --config=lint //...
```

Clippy warnings fail verification. Fix the underlying issue instead of suppressing a diagnostic globally. Preserve independent test oracles: native runtime reports are compared with reference fixtures, support propagation is compared with a fixed-point implementation, and arithmetic results are checked through runtime execution.

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

## Dependencies and performance

Update dependencies explicitly:

```sh
bazel run //:update --config=refresh
bazel mod deps --config=refresh
```

Review both lockfiles and repeat verification. Ordinary builds reject stale resolution data; refresh mode belongs to dependency maintenance. Keep local executor settings in ignored `user.bazelrc` and shared behavior in `.bazelrc`.

Use optimized builds for measurements, preserve the input and limits, and compare observable results before accepting an optimization. The [performance guide](performance.md) documents the runtime benchmark and [arithmetic guide](../mathematics/arithmetic/README.md) documents digit circuits. Keep measured results separate from algorithmic assumptions.
