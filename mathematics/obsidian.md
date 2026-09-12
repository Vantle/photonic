# Obsidian

Obsidian checks a concrete reachability claim: under the supplied Photonic program, can the initial configuration reach this exact target configuration with supported evidence?

Write this schematically as `program ⊢ initial ↝* target`. The star permits zero or more applications, so an initial configuration proves its own reachability. The program includes its declared rules and any code those rules legitimately expose. This is a statement relative to that program, not an assertion that its rules are mathematically valid axioms.

## Run the first claim

```sh
bazel run -c opt //system:command -- obsidian "$PWD/mathematics/natural/addition.wave" --target "$PWD/mathematics/natural/result.particle" --json
```

The initial configuration encodes two plus three. The target is the numeral five. The target file contains only a configuration in the original grammar. It introduces no additional declarations. Native and structured JSON files are supported with the same format selection as `run`.

## Results

| Outcome | Meaning |
| --- | --- |
| `reached` | An exactly equal canonical configuration has supported evidence. The report identifies its witness state. Exploration need not finish once such evidence exists. |
| `unreachable` | Exploration has completely closed and the exact target has no supported occurrence in the graph. |
| `unknown` | Exploration is unfinished without a supported witness. |

The command currently explores the requested budget before reporting; it does not yet stop immediately at the first witness. All three outcomes are successful query execution and use exit status zero. Syntax, target-shape, and I/O failures are command errors. Automation should inspect `outcome` in the JSON report.

`--steps`, `--states`, `--records`, `--coherences`, `--cells`, `--frames`, and `--workers` retain their existing meanings. A limit is not a refutation. Library callers can resume an `obsidian::Search` with `run` or `parallel`, then inspect `report`.

## Exact target meaning

The target is a complete configuration, not a fragment to find somewhere in another configuration. `B.Extra` does not establish a target of just `B`. Particle and coherence order do not matter, but multiplicity, sharing, captured environments, and continuations do.

The first textual target notation describes root coherences and independently introduced occurrences, using the supplied program's root declarations. A displayed `Goal` inside an unfinished body does not establish root-level `Goal`. Likewise, two independently specified target occurrences do not match one inherited occurrence shared across coherences. Textual targets cannot yet describe arbitrary captured-frame graphs or shared introductions; an encoding using the existing language remains future work.

Target compilation uses an isolated clone of the program interner and does not install declarations or alter execution. Targets are exact values, not wildcard patterns or quantified statements.

## What is proved today

The trusted Photonic evaluator establishes the reachability verdict. The JSON report includes the complete source program, target values, witness identifier when reached, and the ordinary execution report: configurations, events, evidence views, read/consume footprints and support status.

This is an inspectable witness report, not yet an independently replayable proof certificate. Source-inferred events may rely on auxiliary derivations; selecting a visually short path is not sufficient to discard their evidence. A portable checker must account for all those dependencies.

The runtime already allows arbitrary rules and circular reasoning. Obsidian does not reject those programs as logically invalid. It reports reachability under their operational and support semantics. It accepts only supported targets as reached; an unfinished search remains unknown. A rule that directly produces a requested answer makes that answer reachable relative to that rule; it does not independently validate the rule as mathematics.

## Path to mathematical proof

1. Use concrete reachability to validate calculations, beginning with addition.
2. Specify an inert representation of propositions, assumptions, and proof objects, plus a fixed trusted calculus/checker program.
3. Use Obsidian to establish the checker's accepted conclusion from the encoded input and certificate under that fixed program. Arbitrary candidate rules must not be allowed to impersonate acceptance.
4. Design a versioned certificate carrying the necessary event, witness, scope, and support evidence, and an independently testable replay procedure. Every certified inference must carry its positive premises.
5. Add quantification and induction to the mathematical calculus. Proving one target for each of several tested numerals does not prove a theorem for every numeral.

Reachability is the first proof judgment. Equality, implication, and induction will be encoded and justified explicitly rather than inferred from arbitrary rewrite edges. Evidence search and checking can both remain ordinary Photonic computation; their different trust roles do not require a separate runtime evaluator.

An Obsidian `unreachable` report describes exhaustive finite exploration from outside the evaluated program. It is not a negative premise, does not produce a language concept, and cannot enable a rule.
