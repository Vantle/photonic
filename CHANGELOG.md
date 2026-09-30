# Changelog

Photonic follows [Semantic Versioning](https://semver.org). [The compatibility contract](document/compatibility.md) says what each release keeps stable.

## 1.0.0

The first stable release.

- The language: atoms, particles and coherences; rules that consume and produce, with every output receiving the remainder; scopes; rules as values and fields; inference. One grammar reads `.wave` programs and `.particle` libraries.
- Exploration: `photonic` explores every future of a program with Laser, the default engine, or with the interpreter, follows one direct path, or explores every schedule of plain events on Laser or on the GPU through Metal.
- Verification: Prism checks exact targets, and Spectrum answers questions about every configuration a program reaches (`check`, `explore`, `select`, `inspect`, `cause`, `miss`, `step`, `compare` and `shape`) on the command line and over the Model Context Protocol.
- The standard library, written in Photonic: fourteen packages, from Boolean tables to unbounded naturals, recursive vectors and expression evaluation.
- Eighty theorems proved by execution, from Boolean algebra to the library's own arithmetic, and five refutations.
- The webbook at [photonic.vantle.org](https://photonic.vantle.org) and the Lightbox run every example live in WebAssembly.
- Prebuilt `photonic` binaries for macOS, Linux and Windows on ARM64 and x86-64, each with the third-party notices it needs.
- Bazel rules for Photonic libraries, programs and tests: `photonic_library`, `photonic_binary` and `photonic_test`.
