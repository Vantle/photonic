# Core design

Types are computation. Molten uses one rule-application mechanism for direct matches, abstraction derivations, and inferred applications at concrete sources. Existing evidence enables additional outgoing applications; it does not rewrite a source's history.

Coherences are independently evolving parallel contexts. An interaction requires one joint witness configuration. Joint siblings may project back to their common source once; results from competing histories cannot be pooled into a witness.

A literal result replaces the concrete matched footprint and preserves unrelated source remainder. A nested body receives the concrete witness not consumed directly by its pattern, together with that remainder. Evidence-only intermediate results are not added to the source's output. Source inference can therefore produce a different alternative from sequentially executing the evidence path.

Unchanged inherited resources share introduction identity. Independently produced results have distinct introductions, even when their labels and causal ancestors agree. Canonical configurations preserve that distinction while sharing repeated evaluation states.

Whole rule values can themselves undergo ordinary consuming replacement. The Rust runtime and JavaScript reference activate finite ground rule values in participating coherences, retaining their lexical captures and conditional availability. Invocation reads the definition; matching it as an operand consumes it. This does not make two rules globally equivalent or add a separate type engine.

Circular dependencies, contrary assertions, and later-defeated inferences remain expressible. Support distinguishes established, conditional, and unsupported derivations. A bounded failure to find evidence is not a proof of absence. There is no semantic depth cutoff or general termination guarantee.

The [configuration semantics](semantics.md) is the current contract for the bounded Rust runtime and JavaScript [laboratory](plan.html). It integrates joint source projection, multiple outputs, nested bodies, canonical states, whole rule values, dynamic activation, and well-founded support. The [implementation plan](plan.md) records remaining work. Rust provides executable [native lowering](syntax.md), configuration graph execution, structured diagnostics, and a bounded CLI, alongside the separate structural parser and literal rule primitive.
