# Mathematics

Build a mathematical library in Photonic, beginning with natural numbers and a small, explicit account of proof. This folder is the research starting point, not an established foundation or a completed theorem library.

Natural numbers are the first test case because zero, successor, recursion, and induction exercise the mechanisms that later definitions will need. Start with numbers and proof foundations together: evaluating a numeral is useful progress, but evaluating examples does not prove a universal theorem.

[Obsidian](obsidian.md) checks exact state reachability under a supplied program and is the first proposed proof judgment. The [plan](plan.md) records the sequence, decisions, and acceptance criteria. The [natural-number walkthrough](natural/index.html) and [formal definition](natural/definition.md) specify the first carrier and connect it to executable checks. No numerical primitives, theorem-specific runtime paths, variable syntax, or constructors are introduced.

## Current status

| Work | Status |
| --- | --- |
| Obsidian concrete reachability | Implemented using the existing evaluator; portable certificates remain planned |
| Unary numeral representation | One Unit atom; empty zero; successor; least closure; written constructor and induction arguments |
| Addition on concrete numerals | [Written laws](natural/law.md), bounded runtime regressions, and a runnable 3 + 7 example |
| Ternary word arithmetic | [All four operations and measurements](arithmetic/README.md); ordinary generated rules, up to 20-trit inputs |
| Ternary sparse addition | [Ordinary carry rules and measured binary comparison](ternary/README.md) |
| Functional numerals | [Fixed-rule streaming successor](arithmetic/stream.md); reusable result construction remains open |
| Unary multiplication | [Repeated actions and written laws](natural/multiplication.md); concrete composition and rule-value checks; numeral-to-action conversion unimplemented |
| Finite nonnegative decimals | [Coefficient/scale specification](decimal/README.md) and executable common-scale addition; alignment and normalization unimplemented |
| Recognition of well-formed numerals | Ordinary membership rules; concrete reachability checks |
| Proposition and proof representation | Design work |
| Equality proof checking | Planned |
| Quantification and induction | Planned |
| Universal arithmetic certificates | Not yet encoded or checked in Photonic |

Photonic intentionally permits arbitrary rules and inconsistent programs. A mathematical result must therefore state its assumptions and carry evidence checked against a designated calculus. The presence of a value named `True`, `Equal`, or `Proven`, or a runtime status of `supported`, does not by itself certify a theorem of that calculus.

Keep definitions, executable experiments, conjectures, and checked theorems clearly identified. Keep the foundation small enough to inspect and its assumptions explicit enough to compare with another foundation.
