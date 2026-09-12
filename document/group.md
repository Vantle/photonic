# Shared-prefix groups

Shared-prefix lowering is implemented. The original grammar accepts `A(B,C)`, and executable lowering now expands it consistently. Its purpose is to avoid repeating A: `A(B,C)` abbreviates `A.B, A.C`. It does not establish a private container or a pairing identity. This clarification supersedes the earlier nested-configuration proposal.

## One flat construction

`Multiply(Number,Number)` expands to `Multiply.Number, Multiply.Number`. Number remains an ordinary literal concept. Each occurrence requires evidence from program rules; neither is a variable, and the concrete witnesses need not be equal. Existing source inference can apply a rule at the concrete source of that evidence.

The abbreviation must elaborate consistently in initial configurations, patterns, and plain outputs. It must preserve the behavior of the fully written form, including empty groups, repeated occurrences, nested parentheses, and depth diagnostics. Scoped rule bodies retain their existing interpretation. Dot composition distributes over alternatives: `(A,B).(C,D)` expands to `A.C,A.D,B.C,B.D`. Parenthesized alternatives can nest; `()` contributes one empty particle. In a rule destination, adjacent destination groups remain separate outputs, as before: `[Seed] (A)(B)` emits A and B in distinct coherences. A one-million-unit frontend budget bounds generated structure and copied text; exhaustion returns a structured expansion diagnostic before materialization.

Repeated source text and runtime broadcasting have different provenance. Two separately written initial occurrences are independent introductions. A runtime split can carry one inherited occurrence into multiple coherences, and reunion reconciles that shared occurrence once. A textual abbreviation must not silently turn independent introductions into a shared resource.

## Many coherences

A two-input rule selects two distinct eligible coherences in an applicable execution context. With four independently introduced, distinguishable operands carrying Multiply, every eligible unordered pair is a possible application. Source adjacency supplies no pairing information. Permutation-equivalent applications and states can be deduplicated; semantically different pairings remain different possibilities.

These applications are alternative successor events, not a promise to execute every overlapping pair together. Each event follows the usual consumption and provenance rules. Independent work can execute in parallel; overlapping resource use must retain the existing event semantics.

If a program requires two particular calculations to remain separate, it must supply that distinction through its existing execution contexts or ordinary labels and rules. Merely assigning the same Multiply atom to all four operands cannot communicate two intended pairs. Introducing hidden pair identifiers would add semantics beyond the abbreviation.

## Whole-operand evidence

The preferred numerical contract is that the complete operand establishes Number. `Unit.Extra` deriving `Number.Extra` does not establish that the entire operand is Number. Conversely, a program may explicitly define a derivation using both Unit and Extra to establish Number; the runtime should not reserve or reject Extra.

Exact complete-state evidence is available through Obsidian, and executable examples demonstrate it. This preference is not an automatic guard in the flat pattern matcher. Ordinary matching is open: a pattern can select part of a coherence, and unmatched contents follow the existing remainder law. Thus `[Multiply.Number, Multiply.Number]` alone does not certify that Number accounts for every other occurrence in each coherence.

Do not silently change every rule to exact whole-coherence matching. That would change existing remainder transfer and composition. Also do not specialize Number or Multiply in the runtime. The remaining question is how a rule can request a complete operand boundary while preserving ordinary open matching elsewhere. Any successful open match still succeeds with an unrelated inert Extra in its coherence; another positive open rule cannot remove that fact. If it requires a general boundary or evidence contract, specify that change explicitly before implementation. Parentheses used only as shorthand do not supply such a boundary.

## Multiplication acceptance

A grouped spelling does not implement multiplication. A single rule `[Multiply.Unit, Multiply.Unit] Unit` consumes two units, emits one, and reunites the remaining units: for positive independent inputs it produces m + n − 1, not their product.

Completion requires one fixed ordinary-rule program taking two runtime numerals, with zero, one, operand permutations, larger inputs, and incorrect pure-numeral targets checked. A coefficient embedded in an output action is still fixed-action scaling. The rejected preparation experiment in [multiplication](multiplication/README.md) demonstrates why a completion marker cannot certify that all input units were processed.

No arithmetic primitive, implicit capture, negative premise, or rule priority is part of this direction. A [retained-input two-numeral construction](../mathematics/natural/product.md) now has executable evidence. It requires helper rule values and an exact independent-product target; the bare grouped multiplier and general whole-operand guard remain unfinished. See the [executable walkthrough](../example/group/README.md) for the implemented behavior and the precise remaining boundary.
