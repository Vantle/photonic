# Runtime rule occurrences

Implementation contract authorized September 22, 2026. This supersedes the alternatives in [the loading assessment](loading.md); its recorded observations remain evidence of the previous behavior. This migration preserves rule definitions. It changes loading, executable availability, resource ownership, and zero-input application.

## Values and ownership

Immutable rule code is interned. Every introduction creates a separate occurrence with a resource identity and lexical capture. Equal code does not imply equal occurrences or captures. A live occurrence belongs to a coherence or an execution context. Captured operands retained as evidence are separate from live context members.

Loading initializes the root context with the written rule occurrences. Entering an output body initializes that body's context. Neither operation introduces the atoms mentioned inside a rule. A compiled scope describes initialization; it never independently authorizes execution.

Context members are visible in their lexical descendants. Visibility exposes the original resource; it does not introduce copies. Coherence members retain their explicit local ownership. Executing reads an eligible live rule occurrence. Explicit inputs consume selected resources, including rule occurrences. Consumption wins over reading and removes access through every live visibility path in the resulting state. Existing histories and other branches retain their own immutable states.

Unmatched context members remain in their owner. They do not join coherence remainders. Ordinary splitting preserves the identities of shared resources; introducing two values creates two identities. A context retires when it is no longer reachable from execution or captures. A retained immutable instruction cannot keep a consumed occurrence executable.

## Input locations

Nonempty input positions can bind values in a coherence or a lexical context. In particular, a rule-valued operand does not require an unrelated coherence. Bindings retain the actual ownership location of every selected resource. Matching through lexical visibility does not allow one resource to fill repeated operands.

Empty particle positions bind actual coherences, including nonempty coherences whose particles remain unmatched. Distinct coherence positions require distinct coherences. Zero positions are different: `[] A` has no operand positions and can execute in the loaded root context. Its application records the enabling occurrence and context, without manufacturing an input coherence. Repeated applications are permitted. A local zero-input rule is read without implicitly selecting its containing coherence as an operand.

The parser's existing interpretation of `[A] [B]` is unchanged. This migration does not introduce bidirectional syntax.

## Migration inventory

| Layer | Required change and evidence |
| --- | --- |
| Compilation and loading | Intern written and emitted rules through one code path; introduce separate live occurrences at root and body entry; test equal code and distinct captures |
| State and identity | Add context members alongside retained operands; include ownership and capture in canonicalization, fingerprints, graph refinement, resource allocation, and reachability |
| Direct evaluation | Replace declaration consumers with live occurrence consumers; support context operands and genuine zero-input searches; invalidate subscriptions after consumption and context reuse |
| Exhaustive evaluation | Discover rules from occurrences; project context operands and rule reads through views; import live captured environments without reviving consumed resources |
| Application and provenance | Consume live aliases consistently, preserve historical evidence, initialize nested occurrences, and account for context resources under limits |
| Exact targets | Specify context membership explicitly in the target contract; audit target construction instead of silently projecting rules out of state equality |
| Reports and inspection | Expose context particles and context resource locations; update native and browser consumers together |
| Programs and libraries | Preserve textual rule contents; audit expected configurations, labels, limits, empty inputs, and library occurrence multiplicity individually |
| Validation | Exercise direct and exhaustive execution, chunked budgets, cache eviction, capture/import, branch preservation, native and WebAssembly builds |
| Measurement | Record initialization, ordinary execution, scope-heavy execution, occurrence churn, retained memory, and release cost against the previous implementation |

## Implementation evidence

The state representation now distinguishes live context particles from held operands. Identity flows, canonical renaming, incidence graphs, structural fingerprints, reachability, resource allocation, and report nodes represent this distinction. Focused regression cases cover equal-code occurrence identity, capture differences, lexical visibility, context projection, retirement, and incremental allocation.

Validation of the representation stage: all 110 Bazel test targets passed across the full run and the targeted report-schema comparison rerun; the kernel now contains 240 passing tests. The pinned-runtime comparison normalizes only the newly introduced empty `frame.particle` field in historical reports. Its other comparisons remain unchanged. Native and browser test targets are included. No loading or executable-availability change is claimed by this checkpoint.

One ownership question is pending: whether consumption should additionally disable shared rule resources in sibling coherences. The broader suite demonstrated that doing so breaks existing map, repeat, and arithmetic programs. This checkpoint preserves their current behavior while implementing removal from an explicitly selected context owner.

The migration remains in progress. This document does not claim that loading, both evaluators, report consumers, or the complete validation matrix have migrated until their checks are recorded.
