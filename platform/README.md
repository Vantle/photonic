# Platform verification

Bazel target platforms specify both architecture and operating system. Linux uses the hermetic glibc 2.28 baseline. Windows targets specify GNULLVM and MSVCRT explicitly. The detected host platform selects MSVC for Windows compiler tools, build scripts, and procedural macros; Cargo resolution includes both Windows ABIs. This keeps procedural macros compatible with the downloaded Rust compiler while producing GNULLVM executables.

| Target | Native CI runner |
| --- | --- |
| `//platform:aarch64-apple-darwin` | `macos-15` |
| `//platform:x86_64-apple-darwin` | `macos-15-intel` |
| `//platform:aarch64-unknown-linux-gnu` | `ubuntu-24.04-arm` |
| `//platform:x86_64-unknown-linux-gnu` | `ubuntu-24.04` |
| `//platform:aarch64-pc-windows-gnullvm` | `windows-11-arm` |
| `//platform:x86_64-pc-windows-gnullvm` | `windows-2025` |

The [verification workflow](../.github/workflows/verify.yml) builds and tests optimized binaries on each configured native runner. A separate Linux job checks Rust formatting through the runtime, arithmetic, and binary formatter targets. Bazel 9.2.0 comes from `.bazelversion`; the workflow pins Bazelisk 1.28.1, checkout 6.0.2, and setup-bazel 0.19.0. Actions use immutable commit references. Cargo is not a workflow command. Runner labels identify operating-system families and architectures, but hosted images receive updates. The labels come from GitHub's [runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners); Bazel bootstrap and caching follow [setup-bazel](https://github.com/bazel-contrib/setup-bazel/tree/c5acdfb288317d0b5c0bbd7a396a3dc868bb0f86).

`--config=native` runs tests on the selected target platform. Use it only when the target matches the machine's architecture and operating system. This allows native Linux tests with the explicit glibc baseline and native Windows tests with a different compiler-host ABI. Ordinary cross-builds retain Bazel's execution-platform checks.

Bazelisk downloads are cached on every runner. Repository downloads are cached on macOS and Linux; Windows repository caching is disabled because its archive step failed after several minutes with files changing during compression. The runtime and arithmetic suites have Bazel's medium test size, allowing five minutes per suite.

From the repository root, cross-compile the CLI without attempting to execute the target binary:

```sh
bazel build //system:command --platforms=//platform:x86_64-unknown-linux-gnu
bazel build //system:command --platforms=//platform:x86_64-pc-windows-gnullvm
```

To verify on a matching native host:

```sh
bazel build -c opt --config=native //... --platforms=//platform:aarch64-apple-darwin
bazel test -c opt --config=native //... --platforms=//platform:aarch64-apple-darwin
bazel run //:format -- --check
bazel run //mathematics/arithmetic:format -- --check
bazel run //mathematics/binary:format -- --check
```

Replace the example target with the matching platform from the table. Cross-compilation alone does not establish native execution; the workflow runs the tests on each matching host.
