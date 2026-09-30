# Contributing

Photonic builds with Bazel alone. Install [Bazelisk](https://github.com/bazelbuild/bazelisk), which runs the Bazel version that `.bazelversion` pins; Bazel then downloads the pinned Rust, LLVM and Node toolchains and every crate. The first build takes a while and fills a disk cache of up to 20 GB in `~/.cache/photonic/bazel`.

## Checking a change

```sh
bazel test -c opt //...
bazel run -c opt //:format
bazel run -c opt //:link
```

Every build also runs rustfmt, Clippy and Buildifier, and a warning fails it. `//:format` formats Rust and Starlark, and `bazel run -c opt //:format -- --check` only checks them. `//:link` checks every link, anchor and Bazel label that the README and the webbook reach. Five tests each need more than 2 GiB of memory and carry the `memory` tag; `--config=continuous` skips them, as CI does, which suits a machine with less than 16 GB.

After changing an example or a listing in the webbook, run `bazel run -c opt //book:record` and commit `book/record.js`; `//book:record.check` fails until the record matches the page.

## Conventions

[AGENTS.md](AGENTS.md) holds the conventions for code, build files and documentation. They apply to people and agents alike.

A commit message is one sentence in the imperative that says what the change does, such as "Explore with Laser on every core by default". Test what users rely on, and keep each document that describes the repository as it is in step with the change. The records under `document/` describe the runtime at their baseline commits and are not updated afterwards.

Changes to a stable interface, as [the compatibility contract](document/compatibility.md) defines it, need an entry in [CHANGELOG.md](CHANGELOG.md).

## Pull requests

The [Verify workflow](.github/workflows/verify.yml) checks every pull request: the test suite natively on each [supported platform](document/build.md#platforms), the browser check, formatting, links and the dependency command. [Continuous verification](document/automation.md) describes it and the Buildkite jobs that also check commits to this repository.

## Reporting a bug

Open an [issue](https://github.com/Vantle/photonic/issues) with the program, the command, the output of `photonic --version` and what you expected. Report security problems privately, as [SECURITY.md](SECURITY.md) describes.

## License

Unless you explicitly state otherwise, any contribution you intentionally submit for inclusion in Photonic, as defined in the Apache-2.0 license, is dual licensed under the MIT and Apache-2.0 licenses, without any additional terms or conditions.
