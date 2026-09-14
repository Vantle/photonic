# Verification

[Webbook](../index.html) · [Repository guide](../README.md)

- [Obsidian](#obsidian)
- [Shared prefixes and complete evidence](#group)
- [Multiplication construction experiment](#counterexample)

<a id="obsidian"></a>

## Obsidian

Obsidian checks a concrete reachability claim: under the supplied Photonic program, can the initial configuration reach this exact target configuration with supported evidence?

Write this schematically as `program ⊢ initial ↝* target`. The star permits zero or more applications, so an initial configuration proves its own reachability. The program includes its declared rules and any code those rules legitimately expose. This is a statement relative to that program, not an assertion that its rules are mathematically valid axioms.

<a id="obsidian-run-the-first-claim"></a>

### Run the first claim

```sh
bazel run -c opt //system:command -- obsidian "$PWD/mathematics/natural/addition.wave" --target "$PWD/mathematics/natural/result.particle" --json
```

The initial configuration encodes two plus three. The target is the numeral five. The target file contains only a configuration in the original grammar. It introduces no additional declarations. Native and structured JSON files are supported with the same format selection as `run`.

<a id="obsidian-results"></a>

### Results

| Outcome | Meaning |
| --- | --- |
| `reached` | An exactly equal canonical configuration has supported evidence. The report identifies its witness state. Exploration need not finish once such evidence exists. |
| `unreachable` | Exploration has completely closed and the exact target has no supported occurrence in the graph. |
| `unknown` | Exploration is unfinished without a supported witness. |

Ordinary graph exploration runs up to the requested budget or closure before reporting. With `--path`, direct execution stops when the current configuration equals the target, when it revisits a configuration, or when a budget prevents further progress. The direct-path mode reports `reached` or `unknown`; it does not establish unreachability of alternative paths. All three outcomes are successful query execution and use exit status zero. Syntax, target-shape, and I/O failures are command errors. Automation should inspect `outcome` in the JSON report.

`--steps`, `--states`, `--records`, `--coherences`, `--cells`, `--frames`, and `--workers` retain their existing meanings. A limit is not a refutation. Library callers can resume an `obsidian::Search` with `run` or `parallel`, then inspect `report`.

<a id="obsidian-exact-target-meaning"></a>

### Exact target meaning

The target is a complete configuration, not a fragment to find somewhere in another configuration. `B.Extra` does not establish a target of just `B`. Particle and coherence order do not matter, but multiplicity, sharing, captured environments, and continuations do.

Textual targets describe root coherences and independently introduced occurrences, using the supplied program's root declarations. A displayed `Goal` inside an unfinished body does not establish root-level `Goal`. Likewise, two independently specified target occurrences do not match one inherited occurrence shared across coherences. Textual targets cannot yet describe arbitrary captured-frame graphs or shared introductions; an encoding using the existing language remains future work.

Target compilation uses an isolated clone of the program interner and does not install declarations or alter execution. Targets are exact values, not wildcard patterns or quantified statements.

<a id="obsidian-what-is-proved-today"></a>

### What is proved today

The trusted Photonic evaluator establishes the reachability verdict. The JSON report includes the complete source program, target values, witness identifier when reached, and the ordinary execution report: configurations, events, evidence views, read/consume footprints and support status.

This is an inspectable witness report, not yet an independently replayable proof certificate. Source-inferred events may rely on auxiliary derivations; selecting a visually short path is not sufficient to discard their evidence. A portable checker must account for all those dependencies.

Programs may contain arbitrary rules, including cycles. Obsidian checks which results their execution and support semantics permit; it does not certify those rules as valid mathematical reasoning. It accepts only supported targets as reached; an unfinished search remains unknown. A rule that directly produces a requested answer makes that answer reachable relative to that rule; it does not independently validate the rule as mathematics.

<a id="obsidian-path-to-mathematical-proof"></a>

### Path to mathematical proof

1. Use concrete reachability to validate calculations, beginning with addition.
2. Specify an inert representation of propositions, assumptions, and proof objects, plus a fixed trusted calculus/checker program.
3. Use Obsidian to establish the checker's accepted conclusion from the encoded input and certificate under that fixed program. Arbitrary candidate rules must not be allowed to impersonate acceptance.
4. Design a versioned certificate carrying the necessary event, witness, scope, and support evidence, and an independently testable replay procedure. Every certified inference must carry its positive premises.
5. Add quantification and induction to the mathematical calculus. Proving one target for each of several tested numerals does not prove a theorem for every numeral.

Reachability is the first proof judgment. Equality, implication, and induction will be encoded and justified explicitly rather than inferred from arbitrary rewrite edges. Evidence search and checking can both remain ordinary Photonic computation; their different trust roles do not require a separate runtime evaluator.

An Obsidian `unreachable` report describes exhaustive finite exploration from outside the evaluated program. It is not a negative premise, does not produce a language concept, and cannot enable a rule.

<a id="group"></a>

## Shared prefixes and complete evidence

These examples run through the Rust frontend, runtime, and Obsidian. Parentheses abbreviate repeated prefixes; they introduce no new grammar, container, variable, or arithmetic operation. The integration suite checks every source/target pair listed here, with closed exploration for these finite examples.

<a id="group-add-three-and-seven"></a>

### Add three and seven

[addition.wave](../example/group/addition.wave):

```text
Add(Unit.Unit.Unit, Unit.Unit.Unit.Unit.Unit.Unit.Unit)
[Add, Add] ()
```

The first line expands to two coherences, each carrying Add. The rule consumes their Add occurrences and reunites their ten independently introduced units.

Run from the repository root:

```sh
bazel run //system:command -- obsidian "$PWD/example/group/addition.wave" --target "$PWD/mathematics/natural/ten.particle"
```

Expected: `Reached`, with closed exploration. Add `--json` for the source, target, witness, and execution graph, or `--workers 4` to run matching work through the worker pool. Parallel execution preserves the result.

<a id="group-match-through-abstractions"></a>

### Match through abstractions

[abstraction.wave](../example/group/abstraction.wave):

```text
Pair(Seed, Other)
[Seed] Intermediate
[Intermediate] Kind
[Other] Kind
[Pair(Kind, Kind)] ([] Result)
```

Each Kind is an ordinary concept derived by rules. The two operands have different concrete contents and different derivation lengths. The joint rule can apply at their concrete source; its body retains Seed and Other. Obsidian reaches `Result.Seed.Other`.

```sh
bazel run //system:command -- obsidian "$PWD/example/group/abstraction.wave" --target "$PWD/example/group/concrete.particle"
```

This is open pattern matching. Adding Extra to Seed does not prevent that pattern from matching; Extra follows the normal remainder law. It is not a complete-operand check.

<a id="group-choose-among-parallel-coherences"></a>

### Choose among parallel coherences

[pair.wave](../example/group/pair.wave):

```text
Pair(A, B, C, D)
[Pair, Pair] Result
```

Every unordered pair is eligible: AB, AC, AD, BC, BD, CD. A selected event consumes two Pair labels and reunites their remainders. The other coherences remain available. Source order establishes no preferred partner.

```sh
bazel run //system:command -- obsidian "$PWD/example/group/pair.wave" --target "$PWD/example/group/selected.particle"
```

This reaches `Result.A.C, Pair.B, Pair.D`. The tests check all six selections. Overlapping matches are alternative events; the existence of six matches does not duplicate each operand into six simultaneous products.

To specify particular independent pairs, [isolated.wave](../example/group/isolated.wave) uses ordinary distinguishing labels:

```text
First(A, B), Second(C, D)
[First, First] Result
[Second, Second] Result
```

```sh
bazel run //system:command -- obsidian "$PWD/example/group/isolated.wave" --target "$PWD/example/group/separate.particle"
bazel run //system:command -- obsidian "$PWD/example/group/isolated.wave" --target "$PWD/example/group/crossed.particle"
```

The intended pairing is reached; the crossed pairing is unreachable after closed exploration. First and Second are ordinary concepts supplied by this program, not runtime identifiers.

<a id="group-check-a-complete-numeral"></a>

### Check a complete numeral

[natural.wave](../example/group/natural.wave):

```text
Check.Unit.Unit.Unit
[Check] Number
[Number.Unit] Number
```

The same two rules accept any finite count of Unit, including zero represented by Check alone. Each Unit can be consumed into Number. Check and Number are ordinary concepts; no numeral parser or builtin numerical type participates.

```sh
bazel run //system:command -- obsidian "$PWD/example/group/natural.wave" --target "$PWD/example/group/number.particle"
bazel run //system:command -- obsidian "$PWD/example/group/extra.wave" --target "$PWD/example/group/number.particle"
```

The first reaches exactly Number. The second starts with an additional Extra and cannot reach exactly Number: it can reach Number.Extra. This is an exact positive reachability question about the complete state, not a negative premise in a language rule. A program can explicitly add a rule accounting for Extra if that is the desired definition.

Obsidian is a proof interface, not an implicit rule guard. A later ordinary `[Number] Result` could still act on Number.Extra and preserve Extra. Integrating complete-operand recognition into rule application requires a general boundary contract that the current flat syntax does not express. No hidden Number-specific check was added.

<a id="group-preserve-resource-identity"></a>

### Preserve resource identity

[broadcast.wave](../example/group/broadcast.wave):

```text
Seed.X
[Seed] A((), ())
[A, A] ()
```

X is inherited by both outputs of the split. Reunion reaches one X. Conversely, [independent.wave](../example/group/independent.wave) begins with `A(X,X)`: those are two independent source introductions, and reunion reaches X.X.

```sh
bazel run //system:command -- obsidian "$PWD/example/group/broadcast.wave" --target "$PWD/example/group/shared.particle"
bazel run //system:command -- obsidian "$PWD/example/group/independent.wave" --target "$PWD/example/group/double.particle"
```

Both are reached. The tests also establish that swapping those targets is unreachable. Shared spelling does not erase the distinction between inheritance and independent introduction.

<a id="group-verification-and-remaining-work"></a>

### Verification and remaining work

```sh
bazel test //system:test
```

Tests cover source, pattern, output and target lowering; Cartesian distribution; nesting; empty and repeated alternatives; rule values; scoped-body diagnostics; bounded expansion; runtime provenance; all six pairings; exact evidence; and CLI execution with one or four workers.

The [retained-input multiplier](arithmetic.md#product) has a two-times-two witness and small closed checks. It requires explicit helper code and an independent-product target; a bare two-argument multiplication interface remains unfinished. Shared-prefix syntax does not turn remainder reunion into a Cartesian arithmetic product. The [design notes](language.md#group) explain what remains to be proved for multiplication.

<a id="counterexample"></a>

## Multiplication construction experiment

Status: rejected candidate. The newer [retained-input construction](arithmetic.md#product) addresses this counterexample and has bounded positive and negative checks. The source in this directory is a reproducible counterexample, not a library implementation.

<a id="counterexample-construction-attempted"></a>

### Construction attempted

[probe.wave](../example/counterexample/product.wave) supplies two independent one-Unit operands, both labelled Multiply. The same declarations accept different operand lengths without embedding a coefficient in an action rule.

Two preparation scopes attempt to convert one operand into X occurrences and the other into Y occurrences. The main scope splits off one work coherence per Y. A work scope turns X into fresh Unit occurrences, while the continuing coherence retains the inherited operand. Scoped cleanup removes the remaining bookkeeping. This attempts to use ordinary rules, lexical scopes, and parallel coherences without capture syntax or arithmetic primitives.

<a id="counterexample-counterexample"></a>

### Counterexample

The candidate reaches two Units for the supplied one-times-one input. Its preparation scopes can return before converting any Unit. Consequently Factor.Unit and Count.Unit can reach the main scope. With no Y, that scope can finish without making a product, passing through the two unconverted Units as its apparent numerical result.

This already invalidates the construction through ordinary execution. Source inference is an additional obligation, not the cause of this particular failure. Scheduling the converter first would conceal an allowed path rather than repair the program's semantics.

Reproduce from the repository root:

```sh
bazel run -c opt //system:command -- obsidian \
  "$PWD/example/counterexample/product.wave" \
  --target "$PWD/example/counterexample/product.particle" \
  --steps 100000 --states 3000 --cells 24 --frames 30 --coherences 8
```

The run reported:

```text
Reached: exact target configuration under the supplied program
Witness s358: [Unit.Unit]@root
942 configurations; 10234 queued, 0 deferred; exploration unfinished
```

A reached counterexample is conclusive even when exploration has not finished. This run does not establish which other results are reachable or whether the candidate's complete exploration terminates.

<a id="counterexample-direct-execution-witness"></a>

### Direct execution witness

The retained report contains this twelve-event path using only direct applications. It does not rely on an inferred shortcut. Frame numbers are omitted here; each preparation and cleanup action still executes in its declared scope.

| Step | Action | Relevant configuration |
| --- | --- | --- |
| 0 | Supply operands | Multiply.Unit, Multiply.Unit |
| 1 | Open the first preparation | Prepare.Unit, Multiply.Unit |
| 2 | Open the second preparation | Prepare.Unit, Prepare.Unit |
| 3 | Start the first return | Finish.Unit, Prepare.Unit |
| 4 | Return the first operand | Factor.Unit, Prepare.Unit |
| 5 | Start the second return | Factor.Unit, Finish.Unit |
| 6 | Return the second operand | Factor.Unit, Count.Unit |
| 7 | Enter the multiplication scope | Begin.Unit.Unit |
| 8 | Begin iteration | Phase.Unit.Unit |
| 9 | Enter cleanup | Clear.Unit.Unit |
| 10 | Start cleanup return | Finish.Unit.Unit |
| 11 | Return from cleanup | Done.Unit.Unit |
| 12 | Return the apparent result | Unit.Unit |

No Unit-to-X or Unit-to-Y conversion occurs. The match on Phase requires positive Phase evidence; it does not require Y to be absent. Likewise, a scope's completion marker is not evidence that every operand occurrence has been processed. The fix cannot consist of scheduling the intended conversions first, adding a negative premise, or trusting a completion label.

<a id="counterexample-remaining-obligation"></a>

### Remaining obligation

Rule-derived Number evidence already enables a body at a concrete source. It does not by itself provide a demonstrated operation that preserves two unknown operand configurations, reuses one in a recursive call, and produces independent copies without admitting incorrect clean numerical results.

The next construction must explain that transport explicitly. The [shared-prefix design](language.md#group) distinguishes concise operand spelling from the unresolved whole-operand evidence contract. Accepting the grouped spelling alone would not establish multiplication. This experiment is not an impossibility proof for the original language.

No capture convention, negative premise, priority rule, or arithmetic-specific runtime behavior was added. The existing action-based multiplication example remains unchanged and retains its documented limitation.
