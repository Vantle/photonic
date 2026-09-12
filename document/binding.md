# Replace or transfer the concrete binding

The [configuration semantics](semantics.md) supersedes the earlier single-result ownership experiment. The Rust runtime and bounded JavaScript [reference](plan.html) integrate source projection with multiple coherences, multiple outputs, nested bodies, fresh introductions, captured rule values, and conditional support. This document summarizes their binding contract.

## Concrete source projection

A match selects occurrences in one joint witness configuration. A relative view maps those occurrences back to their concrete source footprint and tracks the source coherences and environments involved. Several witness outputs may project to one source occurrence. The source is consumed once; independently substituting an ancestor for every pattern position would lose this relationship.

For a literal result, remove the projected introductions from participating source coherences. Reconcile their unmatched remainder by introduction identity, then give each output that remainder and fresh introductions for its explicit result. Unselected coherences are unchanged.

For a nested body, exact source matches are consumed and held by its continuation. Transfer the other concrete witness resources and the unrelated remainder into the body. The body uses the same application mechanism. Unmatched body resources survive return; no Boolean label or operand position has a special disposal rule.

Lexical definition and return destination are separate frame links. A returned explicit result depends on its body's consumed inputs and held enclosing consumption. This makes an aggregate result project to the operator and operands it actually used.

## Examples

| Application at its concrete source | Result |
| --- | --- |
| `True → Boolean` at `True.Extra` | `Boolean.Extra` |
| `B → C` inferred at `A.Extra` through `A → B` | `C.Extra` |
| `B → A` inferred at `A` through `A → B` | `A` |
| `Not.Boolean → body` at `Not.True.Extra` | The body receives `True.Extra`; exact `Not` is held |
| `Not.True → body` at `Not.True.Extra` | The body receives `Extra`; exact `Not.True` is held |
| `Not.Boolean → False` at `Not.True.Extra` | `False.Extra` |
| `[B, C] → D` inferred through `A → [B, C]` at `A.X` | `D.X`, with A consumed once |

For explicit And, the witness `And.Boolean.Boolean.Extra` enables a body at `And.True.False.Extra`. The body receives `True.False.Extra`; `True.False → False` produces `False.Extra`. An aggregate Boolean inference consumes And and both operands, preserving Extra. Intermediate Boolean deductions are evidence, not additional live operands.

## Evidence-only outputs

With `A → B.C` and `B → D`, source inference at A produces D. Executing the producer and then the second rule produces C.D. Both alternatives remain in the graph. Source inference is a semantic operation, not an optimization that may replace an equivalent sequence.

This explicitly replaces the earlier proposal that every sibling produced only inside an evidence path must survive the inferred application. Concrete source remainder survives; evidence-only co-results do not automatically become that remainder. Unrelated intermediate computation also does not enlarge the consumed footprint merely because it appears along an evidence path.

## Identity and support

Two unchanged inherited X references reconcile once. Two independently produced Ys remain independently usable, including when their computations share causal ancestry. Source-flow dependencies therefore cannot serve as a global exclusive ownership key.

Applications retain support for the source, witness, and absence conditions. Equal configurations share storage while incoming events and their conditions remain represented. A speculative witness does not become unconditional through canonicalization.

Whole rule-value replacement follows ordinary consuming replacement in the Rust generic kernel, Rust runtime, and JavaScript reference. The two configuration evaluators also activate finite ground rule values present in participating coherences. A definition is read when invoked and consumed when explicitly matched as an operand.

For generated code, read support and consumed operands are separate. If Seed produces `R: A → B`, sequential execution from Seed.A can reach R.B, while inferred application at Seed.A produces Seed.B. Seed supplied code; only A was consumed. If one Seed jointly produces R and A, the inferred application consumes that Seed through its operand projection and produces B.

The rule and its arguments must coexist in one witness. Alternative histories supplying code and data cannot be combined. A read dependency enables an event at its source; later consumption of the code does not erase an already produced result. Absence-dependent support can still be revised independently.

A body retains the invoked definition's lexical capture without materializing a new rule operand. Captured environments can escape their defining body, and imports preserve introductions shared by their held occurrences. Read support must not enter the body's held consuming footprint.

The implemented higher-order representation uses finite ground constructors and exact whole values. Abstract descriptions can be reached through ordinary derivations. The [native frontend](syntax.md) lowers explicit whole-rule constructors and bodies. Arbitrary partial structural matching and behavioral equivalence are outside the accepted core.

See [configuration semantics](semantics.md) for the normative reference details and [implementation plan](plan.md) for accepted scope and future boundaries.
