# Platform verification

Bazel target platforms specify both architecture and operating system. Linux uses the hermetic glibc 2.28 baseline. Windows targets specify GNULLVM and MSVCRT explicitly. The `windows` host platform preserves the detected architecture and selects the same ABI for compiler tools, build scripts, and procedural macros.

[rust.patch](rust.patch) selects the upstream GNULLVM Rust host distributions in the pinned `rules_rs` dependency. Its default MSVC compiler cannot load GNULLVM procedural macros, while the hermetic LLVM C/C++ toolchain supports the MinGW ABI. Selecting matching compiler distributions keeps the entire Windows build hermetic. The toolchain uses upstream Rust archives because the redistributed archive set omits these Windows host distributions. Compiler archive hashes are recorded in `MODULE.bazel.lock`.

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

Bazelisk and repository downloads are cached on every runner. Windows CI disables the expanded repository contents cache with `--repo_contents_cache=` while retaining downloaded archives. Archiving the expanded contents previously took several minutes and failed because its directory changed during compression. Native jobs stop Bazel before saving caches. The runtime and arithmetic suites have Bazel's medium test size, allowing five minutes per suite.

From the repository root, cross-compile the CLI without attempting to execute the target binary:

```sh
bazel build //system:command --platforms=//platform:x86_64-unknown-linux-gnu
bazel build //system:command --platforms=//platform:x86_64-pc-windows-gnullvm
```

To verify on a matching native host:

```sh
bazel build -c opt //... --host_platform=//platform:aarch64-apple-darwin --platforms=//platform:aarch64-apple-darwin
bazel test -c opt //... --host_platform=//platform:aarch64-apple-darwin --platforms=//platform:aarch64-apple-darwin
bazel test //:check
bazel build --config=format //...
bazel build --config=lint //...
```

Replace the example target with the matching platform from the table. Cross-compilation alone does not establish native execution; the workflow runs the tests on each matching host.

CLI integration tests declare executables as data and resolve `rlocationpath` values with the standard Rust runfiles library. This supports both directory runfiles and Windows manifests.
