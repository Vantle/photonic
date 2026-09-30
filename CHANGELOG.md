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

### Changes since the pre-release builds

Programs, answers and links from builds of `main` before 1.0.0 may need these changes.

- The language
  - Every balanced text is a program. A comma separates members and a missing member is nothing; parts of a term side by side join, so a dot only marks the join and `A(B, C)` is `A.B, A.C`; and a term with brackets is one rule that consumes what its brackets hold, joined, and produces the rest, so `[A] [B] C` consumes `A.B` and `C [A]` is `[A] C`. Parentheses only group inside brackets and particles.
  - Whitespace is Unicode's pattern whitespace and a byte order mark. Control characters, invisible or direction-changing format characters and spaces that are not whitespace are refused inside an atom, and atoms compare by code point. The only other error is an unclosed or unmatched bracket.
  - A JSON program is read only in the shapes its text can write: an object for each program, scope and rule, an array for each particle, and atoms spelled as text spells them. Rules carry no `name`; every rule is shown by its printed text.
  - Diagnostics are `syntax`, `depth`, `expansion`, `library`, `json`, `read` and `encoding`. An unclosed or mismatched bracket is reported where it opens.
  - A rule value firing in a coherence keeps its output in place; only a scope's own rules return output to the scope around it.
- Exploring
  - Configurations with interchangeable parts are named part by part, and every search for a canonical form is charged to the work budget, so programs with many alike scopes, seeds or calls no longer stall.
  - The occurrence limit counts what a configuration's coherences and scopes hold as values, so live rules no longer count, and the scope limit counts only the scopes a run opened.
  - Every open answer names the budget or limit that stopped it and the flag to raise.
  - Laser keeps its work and record budgets inside a round, `--engine metal` bounds the memory of the successors it joins on the host and says whether it ran on the GPU or on the host, and the interpreter's `run --json` keeps only the views its events rest on.
- Spectrum and the command
  - A pattern places each of its parts in a different coherence or scope, `compare` tells apart configurations that differ in their scopes, nesting or live rules, and an exploration is reused only for the same program, naming, mode, engine, budget and goal.
  - Answers name every exploration `closed` or open; `complete` is gone. An open exploration or a direct path never says a rule never fires, and `miss` counts configurations it has not explored.
  - `run` and `prism` list configurations by the handles every question uses, and `prism` exits 1 unless its target is reached. The command exits 0 on success, 1 on a failure or an answer that is not met, and 2 on a command line it cannot parse.
  - `--goal` and `--preserve` work with every question; `--preserve` needs `--exact` or `--goal`.
  - The protocol server reads only files inside the directory it starts in, and its errors name their code and the argument at fault.
  - Each library or program file loads once, and a file given both as a library and as a program is refused.
- The standard library
  - The linked engines of chains, naturals, expressions and vectors no longer take their own pending requests, so every schedule of their plain events ends at the answer, not only the direct path. Their internal words changed.
  - Each package lists the words it reserves, which a program's own data must not use.
- Tests and tools
  - `photonic_library`, `photonic_binary`, `photonic_test` and the documented library targets can be used from other Bazel modules.
  - A failing `photonic_test` says what it explored and prints a `photonic check` command that asks the same question; its sources and targets are parsed when it builds, and Laser runs under its work budget.
  - Seven natural-number theorems gained every-schedule tests, and 76 of the 80 theorems end at `Theorem` in every schedule of plain events.
- The webbook and the Lightbox
  - Share links carry the program in a versioned fragment (`lightbox.html#1&source=…`); links in the old form still open. The Lightbox keeps the draft across reloads and loads any file of the standard library.
- The program optimization learner is experimental and outside the compatibility contract.
