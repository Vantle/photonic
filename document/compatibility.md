# Compatibility

Photonic is in alpha. It follows [Semantic Versioning](https://semver.org), under which a release before 1.0.0 may change anything, so any alpha release may change what this contract covers. [CHANGELOG.md](../CHANGELOG.md) records every such change and what a program, command or link needs in order to follow it.

1.0.0 will make the covered interfaces stable: from then on a major release may break them, a minor release adds without breaking them, and a patch release fixes.

## Covered

- The language: the grammar, the forms and the meaning of a program as [the webbook's reference](https://photonic.vantle.org/#reference) and [the primer](../spectrum/primer.md) state them. A fix that brings the runtime back to this contract is not a break.
- Answers: a closed exploration's configurations, events and verdicts, and a claim's holds, fails or unknown. A later release may settle a claim that an earlier one left unknown, but reverses a settled answer only to fix a bug.
- Program files: `.wave` and `.particle` source, and the JSON program `photonic lower` writes and every command reads.
- The standard library: its packages, namespaces, requests and answers as [its reference](../library/README.md) documents them.
- The `photonic` command: its verbs, flags and exit codes. Exit status 0 means success; 1 means a failure, or a claim, comparison, shape or Prism target the answer did not meet; 2 means the command line could not be parsed.
- Spectrum's JSON envelope, version 1, and the Model Context Protocol server's tools, their input schemas and the primer resource, as [the Spectrum contract](spectrum.md) describes them. A release may add fields to answers and optional fields to requests without changing the envelope's version.

## Not covered

- Text written for people: the layout of the command's plain output, diagnostics and help.
- Exploration keys, which hash the engine, the mode and the budgets; handles, which are stable within one exploration; work counts, timings and memory.
- Budget defaults and the default engine, which a release may change to settle more claims.
- The execution reports of `photonic run --json` and `photonic prism --json`, which follow the runtime.
- The Rust crates, which are not published, the benchmarks, the program optimization learner, the webbook's engine protocol and the dated records under `document/`.
