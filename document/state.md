# Canonical states and coherence outputs

The [configuration semantics](semantics.md) is the current contract. The Rust runtime and [JavaScript laboratory](plan.html) integrate configuration sharing, joint source projection, resource allocation, captured rule values, nested frames, and positive evidence closure. Earlier local-state and tuple experiments remain historical references.

## Share computation, preserve structure

State content is orderless: A.B equals B.A, while A.A retains independent multiplicity. A configuration contains coherences and their reachable frames. An empty coherence differs from no coherence.

Canonicalization renames anonymous introductions and frames while preserving their incidence graph: which occurrences share an introduction, which coherence holds each occurrence, and which frames define lexical scope, return destinations, and held resources. Equal labels alone are insufficient. Two equal explicit output coherences remain two positions even when their contents share storage.

The reference interns finite ground rule structures into code keys within one model instance. Live rule values and their captured frames participate in configuration identity. Canonicalization follows capture edges from live and held values, preserves their sharing, and retains reachable frames even after a closure escapes. It handles cyclic capture graphs; equal code with distinct captured structure does not collapse into one executable value.

Support remains in clauses over states, events, views, and queries; sharing a state does not discard its alternative conditions. Code keys are model-local, so caches are not reused across unrelated compiled programs.

A → A may close as a self-loop. A → B → A can return to a shared configuration. These graph cycles do not place an old source beside its successor as a second live input.

## Shared remainder and fresh results

For the atomic rule `[A, B] → [C, D]`:

```
[A.X, B.Y] → [C.X.Y, D.X.Y]
[C, D] → E
Result: E.X.Y
```

Both outputs carry references to the same X and Y introductions, so reunion preserves each once. Independently introduced equal Xs remain X.X. Removal applies across the complete participating binding: consuming an X alias prevents another participating alias from restoring it. Unselected coherences are not mutated.

Explicit outputs receive fresh introductions. Therefore:

| Branch behavior before reunion | Carried result |
| --- | --- |
| Both carry the same inherited X unchanged | X |
| One computes X → Y, the other X → Z | Y.Z |
| Both independently compute X → Y | Y.Y |
| Y is computed once before divergence | One shared Y |

An explicit X → X is also production. In an isolated coherence it can be alpha-equivalent to its input; alongside another coherence retaining the original X, the changed sharing pattern distinguishes the result. Causal ancestry is not a global exclusive consumption permission.

## Joint evidence

Match within a complete reachable configuration. A Cartesian product of historical states can combine incompatible alternatives and is not a witness.

If A jointly produces B and C, `[B, C] → D` can project back to A once. If `A → B` and `A → C` are competing alternatives, there is no joint B/C configuration. The same ancestor label does not decide these cases; the producing event and its relative flow do.

Source inference also preserves its own output law. With `A → B.C` and `B → D`, inferred D and sequential C.D remain distinct alternatives. Evidence-only co-results are not copied into a concrete source result automatically.

## New evidence and matching gates

Intern configurations, events, relative views, and support clauses independently. A new view can enable an application at an already known source. Equivalent evidence extends support for an existing application instead of allocating another result. Only positively established evidence enables computation; a cycle cannot establish its own missing premises.

Both evaluators cache matching by target configuration, frame, and pattern. Both retain lazy enumeration progress; Rust incrementally delivers completed cached bindings to subscribers and releases the candidate search when finished. Compatible occurrence assignments retain their identities; repeated reports do not supply another operand. Different relative views can reuse those cached bindings while projecting their own source footprints. Adjacency indexes schedule related work without pooling incompatible configurations. Rust can advance independent search and normalization steps on a Rayon pool, then merge their results in coordinator queue order.

Generated definitions activate locally and retain read support separately from consuming flow. Established historical evidence is preserved when later events consume or replace code. There is no absence-dependent invalidation.

## Growth and bounds

Unchanged inherited context can close a divergence/reunion cycle:

```
A.X → [B.X, C.X] → A.X
```

Independent branch production can instead create genuinely larger configurations:

```
A.X → [B.X, C.X]
both branches compute X → Y
[B, C] → A
A.Y.Y
both Ys compute Y → X
A.X.X
```

The larger multiplicity is observable and cannot be erased by memoization. Writing rules in both directions does not make them inverses. Budgets suspend unfinished work; they do not establish a semantic depth limit.

Initial configurations contain independently introduced occurrences, so Rust sorts their coherences and renames once without factorial initial-world enumeration. General configurations refine graph colors using sharing, capture, parent, and lexical edges before exact canonicalization. Its resumable search advances one ordering/candidate at a time and reuses completed normalization slots. Worst-case CPU work remains factorial. Matching caches and adjacency indexes reduce repeated work, but finite checks of identity, sharing, scope, projection, and support do not establish production performance or universal completeness. The Rust graph runtime and executable textual lowering are implemented for this finite ground fragment. The accepted core includes resumable particle matching and candidate ordering. Arbitrary partial structural operations are outside this milestone; graph setup, source compilation, and closure normalization are not strictly preemptible. The integrated suite exercises finite ground dynamic rules, capture identity, support, and cache behavior.

The soft retained-record budget is checked between coordinator batches and can overshoot within a batch. Snapshots report current and peak counts; they do not measure exact bytes or provide reloadable checkpoints. Support is solved by dependency component and cached for unchanged snapshots. See [runtime](runtime.md) for the execution controls.

Native execution uses the [original grammar](syntax.md). Whole-rule generalization is documented [here](generalization.md); arbitrary structural substitution is not implemented. The language has no negative premises.
