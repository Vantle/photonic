<p align="center">
  <a href="https://photonic.vantle.org">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="book/logo/dark.svg">
      <img src="book/logo/light.svg" alt="Photonic" width="112" height="112">
    </picture>
  </a>
</p>

<h1 align="center">Photonic</h1>

<p align="center">Write the rules. Explore every outcome.</p>

<p align="center">
  <a href="https://photonic.vantle.org"><b>Read the webbook</b></a>
  &nbsp;·&nbsp;
  <a href="https://photonic.vantle.org/lightbox.html"><b>Open the Lightbox</b></a>
  &nbsp;·&nbsp;
  <a href="https://github.com/Vantle/photonic/releases/latest"><b>Download</b></a>
  &nbsp;·&nbsp;
  <a href="library/README.md">Standard library</a>
  &nbsp;·&nbsp;
  <a href="theorem/README.md">Theorems</a>
  &nbsp;·&nbsp;
  <a href="document/README.md">Documentation</a>
</p>

<br>

Photonic is a language of rules. A program is data and the rules that change it. The runtime explores every configuration those rules can reach, and Prism checks each result exactly: a target is reached, unreachable or unknown.

```
Light,
[Light] Red,
[Light] Green,
[Light] Blue
```

Three rules can consume `Light`, so this program has three futures. The runtime follows all of them and finds four configurations: `Light`, then `Red`, `Green` or `Blue`.

The whole grammar is six characters and five laws, with no keywords. A comma separates, parts side by side join (`A.B` and `A B` are one particle, and `A(B, C)` is `A B, A C`), and a term with brackets is a rule that consumes what they hold and produces the rest. Parentheses group: standing alone, a group that lists a rule is a scope, and joined to a particle, a rule is a value. Every balanced text is a program, unless an atom holds a character the [primer](spectrum/primer.md) refuses, such as a control character or a no-break space; the primer states the laws.

The standard library is written in Photonic, and its theorems are proved by running them in every case. The runtime is Rust, and it also runs in the browser: every example in the webbook is live.

## Install

Download `photonic` for macOS, Linux or Windows from the [latest release](https://github.com/Vantle/photonic/releases/latest), check the archive against `SHA256SUMS`, and put the command on your `PATH`. Releases run on macOS 13 or later, on Linux with glibc 2.28 or later, and on Windows 10 or later, each on ARM64 and x86-64. macOS quarantines a binary downloaded in a browser; `xattr -d com.apple.quarantine photonic` releases it.

To build from a checkout instead, install [Bazelisk](https://github.com/bazelbuild/bazelisk), the only tool you need. It runs the pinned Bazel, which downloads the compilers and every dependency. Then:

```sh
bazel run //:install
```

This puts an optimized `photonic` in `~/.local/bin`, or in `~/bin` or `~/.bin` when one of those is on your `PATH`, and tells you how to add the directory if it is not. Name another directory after `--`. Delete the file to uninstall.

## Try it

The [Lightbox](https://photonic.vantle.org/lightbox.html) runs Photonic in your browser with nothing to install: write a program, run it, and explore every configuration it reaches. Its links carry the whole program, so you can share what you write.

With the command installed, save the program above as `light.wave` and ask what it does:

```sh
photonic explore light.wave
photonic check light.wave --reach Red --avoid Red.Green
photonic check light.wave --plain --end Red
```

`explore` summarizes every future: four configurations, three of them ends, and how often each rule fires. `check` answers claims with holds, fails or unknown, and exits 1 unless every claim holds. The first check holds; the second claims that every run ends at `Red` and fails, because a run can end at `Green` instead. `--plain` follows every order in which the rules can fire, without inference, and `--engine metal` explores those orders on the GPU of a Mac with Metal 3, so programs whose orders reach millions of configurations finish in seconds.

Agents ask the same questions over the Model Context Protocol. Add the server to a client's configuration:

```json
{ "mcpServers": { "photonic": { "command": "photonic", "args": ["mcp"] } } }
```

[The Spectrum contract](document/spectrum.md) describes every question and answer. From a checkout, every command also runs as `bazel run -c opt //command:photonic -- explore light.wave`, and the repository's `.mcp.json` starts the server that way once the first build finishes.

## Develop

Every build also checks formatting and lints, and warnings are errors.

```sh
bazel test -c opt //...
bazel run -c opt //:format
bazel run -c opt //:link
```

Serve the webbook from a checkout with `bazel run -c opt //book:serve`, then open http://127.0.0.1:8080. After changing an example in the webbook, record its runs again with `bazel run -c opt //book:record`; `//book:record.check` fails until the record matches the page. [CONTRIBUTING.md](CONTRIBUTING.md) describes how to contribute, and [SECURITY.md](SECURITY.md) how to report a vulnerability. Report bugs in the [issues](https://github.com/Vantle/photonic/issues).

## Versions and license

Photonic is in alpha and follows [Semantic Versioning](https://semver.org), so any release before 1.0.0 may change the language and its tools. [The compatibility contract](document/compatibility.md) says what 1.0.0 will keep stable, and [the changelog](CHANGELOG.md) what each release changes.

Photonic is licensed under either the [Apache License, Version 2.0](LICENSE-APACHE) or the [MIT license](LICENSE-MIT), at your option. Release archives also carry the notices of the third-party crates the command contains.
