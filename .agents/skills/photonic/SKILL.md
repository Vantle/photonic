---
name: photonic
description: Write, check and debug Photonic programs with Spectrum, the analyzer behind the photonic check, explore, cause, miss and compare commands and the photonic MCP tools. Use when writing or changing .wave or .particle files, when a photonic_test fails, or when asked why a program does or does not reach a configuration.
---

# Photonic with Spectrum

Photonic programs are rules over particles, and the runtime explores every configuration they reach. Spectrum answers questions about that exploration. Read `spectrum/primer.md` (the `photonic://primer` resource) for the grammar and meaning before writing rules, and `library/README.md` before using a library protocol. `document/spectrum.md` is the full contract.

## Loop

1. `check` the program with the claims the task implies, such as `--reach`, `--avoid` or `--outcome`. Fix diagnostics first.
2. `explore` it. Read the end configurations and each rule's activity: a rule that fires `never` is a finding.
3. Explain what surprised you. `cause s11` gives the path to a configuration, `cause s11.o1` where an occurrence came from, and `inspect e12` an event's match, witness and deduction.
4. Explain what is missing. `miss --target False.Extra` lists the nearest configurations and what they lack; `miss r3` shows why a rule never fires.
5. After an edit, `compare` the old and new programs with the same claims, and confirm that only the intended configurations and events changed.
6. Turn the finding into a `photonic_test` so it stays checked.

Over MCP the verbs are tools with the same fields. Pass the `exploration` key from an answer instead of the program to ask more questions of the same recording.

## Honesty

- `unknown` means a budget stopped the search. It is never evidence that a configuration is unreachable. Say which budget stopped it, raise it, or change the question.
- Patterns match by containment, as a rule's input does. Use `--exact`, with `--preserve` when the target must list the root rules, to compare whole configurations as Prism and `photonic_test` do.
- A handle belongs to one exploration. Quote it with its key, and explore again after an edit before reusing any handle.
- Direct paths (`--path`) follow the program as written and number handles in source order; exhaustive explorations use canonical order.

## Pitfalls

- Every output of a rule receives the whole remainder, so a rule with several outputs duplicates what it did not match.
- Particles are unordered: `Inspect.1.2` is `Inspect.2.1`, and a rule's input matches any particle that contains it.
- A live rule value such as `([Digit] 1)` fires on a bare `Digit` in the same coherence; never use such atoms as labels.
- Answer labels are shared by every caller in a frame. Guard an answer with a token that exists only while your own call is in flight.
- Prism reaches intermediate configurations too, so a negative test must name a terminal configuration.
- Programs that build chains or vectors do not close under exhaustive exploration; check them with `path = True`.

## Commands

```sh
bazel run -c opt //command:photonic -- check program/language/conjunction.wave --reach False.Extra --exact --preserve
bazel run -c opt //command:photonic -- explore program/language/conjunction.wave
bazel run -c opt //command:photonic -- cause program/language/conjunction.wave s9.o0
bazel run -c opt //command:photonic -- miss program/language/conjunction.wave r1
bazel run -c opt //command:photonic -- compare before.wave after.wave --reach False.Extra
bazel run -c opt //command:photonic -- shape library/boolean/and.particle library/boolean/or.particle
```

Paths are relative to where `bazel run` starts, and `--json` prints each answer as data. `check` exits 1 unless every claim holds, and `compare` unless the programs behave alike.
