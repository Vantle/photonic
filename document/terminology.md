# Terminology

Rules describe computation, including abstraction derivations. Events record applications. The [configuration semantics](semantics.md) defines their current reference behavior; the [core design](core.md) summarizes the direction.

| Term | Meaning |
| --- | --- |
| Concept | A value carried by an occurrence, including an atom such as `Ready` or a whole ground rule value. |
| Particle | A multiset of live occurrences within one coherence. Independent equal values retain multiplicity. |
| Coherence | An independently evolving parallel context. |
| Configuration | Jointly available coherences with their reachable lexical environments and continuations. |
| Rule | Describes input matching and output behavior; a whole rule can also be a value matched by another rule. |
| Closure | An executable rule value with its captured lexical environment. |
| Read support | Evidence enabling access to a definition without consuming its source as an operand. |
| Event | An application at a concrete source configuration, with its binding, output flow, and support. |
| Introduction | Identity created for an explicit result, preserved when that result is carried unchanged. |
| Occurrence | A local reference to an introduction within a coherence or held continuation. |
| Relative view | A joint derivation with flow from target occurrences to concrete source occurrences. |
| Footprint | Concrete source occurrences selected by projecting a witness binding. |
| Remainder | Unmatched concrete source resources carried to an application's outputs. |
| Frame | A lexical environment and continuation record with separate definition and return links. |
| Support | Conditions establishing a state, event, view, or absence query; classified as supported, conditional, or unsupported. |
| Gate | Compatible partial slot bindings within one joint configuration and frame. Cached match enumeration can reuse them across relative views. |

Use **input**, **output**, and **apply** for rule operations. **Divergence** creates several coherences; **decoherence** combines compatible coherences. These describe ordinary rule behavior. Multiset union remains **merge**.

**Canonical state** means shared configuration content with anonymous identity normalized while preserving multiplicity and sharing. **History** means the event record; **version** may identify a historical coherence occurrence. A repeated canonical state is not a second live copy of its history. **Origin**, when used in older experiments, refers to introduction or dependency information; it is not a universal exclusive ownership law.

A **rule-value rewrite** consumes the matched whole value and produces its replacement. It does not assert behavioral equivalence or globally alias two names. The Rust runtime and JavaScript reference activate produced ground rule values locally, preserving lexical capture and read dependence. Exact whole-value matching and ordinary derivations can describe rules without decomposing their internal fields. `[[A,B]] ([B])` parses structurally, but that abbreviation is not executable native syntax. Use an explicit `@([A] -> B)` constructor; see [frontend contract](syntax.md).

The Rust `rule::Rule::apply` implements generic literal multiset replacement, including exact whole nested rule values. It is separate from `runtime::Runtime`. That runtime and the JavaScript reference integrate joint inference, allocation, nested frames, dynamic activation of finite ground rule constructors, and conditional support. The compiled code universe is fixed within each model instance; visible rule occurrences can change through ordinary events.

Source grammar rules and mathematical relations retain their conventional meanings. Earlier Molten documents used relation for rule, world/partition for coherence, and join for decoherence. Preserve historical or scientific terms when describing their original subjects.

Molten's coherence and decoherence are language-specific names, not a claim to simulate quantum physics. The [research report](research.html) records the physical comparisons and sources.
