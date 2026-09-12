# Platform verification

Bazel target platforms specify both architecture and operating system. Linux uses the hermetic glibc 2.28 baseline. Windows specifies GNULLVM and MSVCRT explicitly; the separate `windows` host platform preserves the detected host architecture for tool execution.

| Target | Native CI runner |
| --- | --- |
| `//platform:aarch64-apple-darwin` | `macos-15` |
| `//platform:x86_64-apple-darwin` | `macos-15-intel` |
| `//platform:aarch64-unknown-linux-gnu` | `ubuntu-24.04-arm` |
| `//platform:x86_64-unknown-linux-gnu` | `ubuntu-24.04` |
| `//platform:aarch64-pc-windows-gnullvm` | `windows-11-arm` |
| `//platform:x86_64-pc-windows-gnullvm` | `windows-2025` |

The [verification workflow](../.github/workflows/verify.yml) builds, tests, and checks formatting on each configured native runner. It pins Bazel 9.2.0, Bazelisk 1.28.1, checkout 6.0.2, and setup-bazel 0.19.0; actions use immutable commit references. Cargo is not a workflow command. Runner labels identify operating-system families and architectures, but hosted images receive updates. The labels come from GitHub's [runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners); Bazel bootstrap and caching follow [setup-bazel](https://github.com/bazel-contrib/setup-bazel/tree/c5acdfb288317d0b5c0bbd7a396a3dc868bb0f86).

From the repository root, cross-compile the CLI without attempting to execute the target binary:

```sh
bazel build //system:command --platforms=//platform:x86_64-unknown-linux-gnu
bazel build //system:command --platforms=//platform:x86_64-pc-windows-gnullvm
```

To verify on a matching native host:

```sh
bazel build //...
bazel test //...
bazel run //:format -- --check
```

Local verification has run the Rust suite on ARM64 macOS and successfully cross-linked the CLI for x86-64 GNU Linux and x86-64 GNULLVM Windows. Those cross-builds do not establish native execution on Linux or Windows. The six-runner workflow is configured but has not been triggered in this checkout.
