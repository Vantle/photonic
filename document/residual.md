# Concrete-source remainders

The [configuration semantics](semantics.md) defines the current remainder law. The Rust runtime and JavaScript [laboratory](plan.html) integrate it with joint source inference, multiple outputs, nested bodies, captured rule values, and conditional support. The earlier overlapping-description and single-result experiments are historical steps.

A literal application replaces its concrete matched footprint and carries unmatched resources from the participating source coherences. `A → B` enables `B → C` at `A.Extra`, producing `C.Extra`. It does not retain A as another live operand.

Remainder is reconciled by introduction identity before broadcasting it to the outputs. Two inherited references to one X contribute one X; independent X introductions retain multiplicity. Consuming a shared introduction through one participating input prevents another participating alias from restoring it. Unselected coherences remain unchanged.

A nested body transfers the concrete witness not consumed directly by its pattern, together with the remainder. `Not.Boolean` matched at `Not.True.Extra` holds Not and transfers True.Extra. A direct Not.True match transfers only Extra. Held consumption participates in the eventual return's source projection.

Evidence-only results are not source remainder. From `A → B.C` and `B → D`, inferred application at A yields D, while sequential execution yields C.D. The evaluator retains both alternatives.

Invoking a generated rule reads its availability. Its read-only source support remains outside the consumed operand footprint: a rule derived from Seed can turn Seed.A into Seed.B through source inference. Explicitly matching the whole rule value instead consumes it. Code and arguments must have one joint witness.

See the [binding contract](binding.md) for worked cases. Rust's `rule::Rule::apply` remains a literal multiset primitive: it broadcasts counted leftovers without introduction reconciliation or graph execution. `runtime::Runtime` implements those configuration-level responsibilities separately.
