# Multiplication construction experiment

Status: rejected candidate. The newer [retained-input construction](../../mathematics/natural/product.md) addresses this counterexample and has bounded positive and negative checks. The source in this directory is a reproducible counterexample, not a library implementation.

## Construction attempted

[probe.wave](probe.wave) supplies two independent one-Unit operands, both labelled Multiply. The same declarations accept different operand lengths without embedding a coefficient in an action rule.

Two preparation scopes attempt to convert one operand into X occurrences and the other into Y occurrences. The main scope splits off one work coherence per Y. A work scope turns X into fresh Unit occurrences, while the continuing coherence retains the inherited operand. Scoped cleanup removes the remaining bookkeeping. This attempts to use ordinary rules, lexical scopes, and parallel coherences without capture syntax or arithmetic primitives.

## Counterexample

The candidate reaches two Units for the supplied one-times-one input. Its preparation scopes can return before converting any Unit. Consequently Factor.Unit and Count.Unit can reach the main scope. With no Y, that scope can finish without making a product, passing through the two unconverted Units as its apparent numerical result.

This already invalidates the construction through ordinary execution. Source inference is an additional obligation, not the cause of this particular failure. Scheduling the converter first would conceal an allowed path rather than repair the program's semantics.

Reproduce from the repository root:

```sh
bazel run -c opt //system:command -- obsidian \
  "$PWD/document/multiplication/probe.wave" \
  --target "$PWD/document/multiplication/wrong.particle" \
  --steps 100000 --states 3000 --cells 24 --frames 30 --coherences 8
```

The run reported:

```text
Reached: exact target configuration under the supplied program
Witness s358: [Unit.Unit]@root
942 configurations; 10234 queued, 0 deferred; exploration unfinished
```

A reached counterexample is conclusive even when exploration has not finished. This run does not establish which other results are reachable or whether the candidate's complete exploration terminates.

## Direct execution witness

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

## Remaining obligation

Rule-derived Number evidence already enables a body at a concrete source. It does not by itself provide a demonstrated operation that preserves two unknown operand configurations, reuses one in a recursive call, and produces independent copies without admitting incorrect clean numerical results.

The next construction must explain that transport explicitly. The [shared-prefix design](../group.md) distinguishes concise operand spelling from the unresolved whole-operand evidence contract. Accepting the grouped spelling alone would not establish multiplication. This experiment is not an impossibility proof for the original language.

No capture convention, negative premise, priority rule, or arithmetic-specific runtime behavior was added. The existing action-based multiplication example remains unchanged and retains its documented limitation.
