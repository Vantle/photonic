# Initial executable rules

Status: design assessment and migration proposal, not an approved semantic change. The user requested assessment before implementation. Runtime optimization continues to preserve the existing language contract. This proposal does not construct, specialize, reconstruct, or replace rule definitions.

## Recommendation

Use one authoritative executable occurrence if written and produced rules are intended to be ordinary values with the same behavior. Do not add runtime copies while retaining an independently executable declaration. Consuming an occurrence must not leave an invisible declaration supplying the same authority.

The decisive choice is availability: local to a coherence, or inherited through an environment. The current implementation has both, with different behavior. Making declarations into ordinary values selects local availability unless a separate language change defines otherwise. This assessment recommends the local model conditionally on that being the intended design; it does not infer that intention from implementation uniformity alone.

Removing the startup `()` is insufficient justification for this migration. If that is the only requirement, defining a default initial coherence is a smaller language change, although it does not unify declarations and values and still needs explicit behavior for empty programs, libraries, and higher input arities.

## Verified baseline

Baseline: `0361a20`. The [observation artifact](loading.json) records executable hashes, source, budgets, and complete reports. These observations establish existing implementation behavior, not the intended language specification. In the table, `R` denotes the actual executable rule resource; it is explanatory notation, not a new atom or syntax.

| Source | Existing behavior |
| --- | --- |
| `[] A` | Installs a declaration; initial configuration has no coherence and no transition |
| `([] A)` | Introduces one rule value in one coherence; `R` can transition to `R.A`, then `R.A.A` |
| `([,] A)` | One coherence cannot satisfy two input slots |
| `([,] A),()` | Two coherences can merge into one containing `R.A` |
| `([,,] A),(),()` | Three coherences can merge into one containing `R.A` |
| `([A] B).A` | Local rule reads `R`, consumes `A`, produces `R.B` |
| `([A] B),A` | Cannot apply: the rule and required data occupy different coherences, while the rule has one input slot |
| `([A] (B,C)).A` | Produces two coherences, `R.B` and `R.C`, carrying the same rule resource identity |
| `([A]).A` | No output coherence remains; the rule resource disappears with its coherence |
| `([A] ()).A` | One output coherence remains, containing the unmatched rule resource |

An empty input slot means no required particle, not a predicate requiring an empty coherence. Slots select distinct existing coherences, and unmatched particles follow the existing remainder rule. Loading a rule value supplies one coherence only if the placement rule creates one. It cannot supply all the distinct sites required by `[,]`, `[,,]`, or an arbitrary larger input. None of these facts makes `[]` a once-only initializer.

For an n-slot empty input under local availability, a rule-only coherence plus n minus one empty coherences supplies n distinct sites. The rule-only coherence is not literally empty: it contains the enabling resource. This proposal therefore does not establish a transition from a configuration with zero coherences. That would require a separate rule for execution without an existing site.

The current parser reads `[A] [B]` as an A-input rule producing a rule value with B input and no output. Establishing `A ↔ B` needs a separate syntax and semantic decision. It is not a consequence of initial rule loading.

## Architectural alternatives

| Model | Benefit | Semantic cost |
| --- | --- | --- |
| Ordinary local rule values | Written and emitted occurrences use the existing local read, match, capture, and provenance machinery | Declaration visibility, placement, lifetime, resulting configurations, and libraries change |
| Environment-owned executable occurrences | Can retain lexical visibility while giving execution an explicit occurrence dependency | Environment placement and consumption need a new contract; ordinary local values still have a different location; no automatic coherence exists for startup |
| Declarations with a default initial coherence | Directly addresses declaration-only startup | Retains two sources of rule availability and changes startup behavior without giving rules ordinary value behavior |

One occurrence representation can support several explicit locations, but a common struct does not make their semantics identical. An environment-owned occurrence is not automatically an ordinary coherence operand. Likewise, giving a local rule scope-wide reach would change which worlds must participate in an application and which evidence enables it.

## Decisions required before implementation

| Decision | Proposed direction for the local model | Unresolved consequence |
| --- | --- | --- |
| Placement | Follow explicit particle and coherence structure; avoid silently placing each rule in a separate coherence | Define what mixed top-level source and imported declarations mean; existing lowering separates them and cannot reconstruct every original position from the lowered program |
| Ownership | Preserve lexical capture separately from physical location | Specify whether moving a rule affects its visibility; capture alone must not create availability |
| Application | Read the enabling occurrence, retaining ordinary input consumption | Reading is not a promise of perpetual existence; no-output transitions can remove the containing coherence |
| Explicit matching | Use ordinary rule-value matching and resource consumption | Define duplicate equal-code occurrences, capture-sensitive matching, and simultaneous read/consume behavior consistently with existing provenance |
| Merge and split | Preserve current remainder identity and flow unless separately changed | A resource carried into two outputs is shared, not two independent fresh resources; branch evidence must remain distinguishable |
| Observation | Keep exact configuration reachability exact, including rule values | `A` and `R.A` differ; any data-only query needs an explicit projection contract and must not silently replace exact reachability |
| Nested scope and library loading | Introduce written rules through the same occurrence mechanism as produced rules | Define recursive groups, body entry/return, import placement, loading multiplicity, and consumption before removing lexical declarations |

Code identity, occurrence identity, and capture identity remain distinct. Interning equal immutable definitions must not merge separately introduced executable resources. Loading two equal rules must follow a specified multiplicity rule, rather than whatever deduplication the compiler happens to perform.

Wrapping an entire existing program in parentheses is not a general migration algorithm. In value position, current lowering combines the group's rules with every initial particle, and state construction allocates an occurrence for each resulting token. This can introduce separate equal-code resources across coherences, rather than the shared resource produced by splitting one coherence. Placement and multiplicity must be chosen deliberately.

## Migration sequence

1. Ratify examples for the decision table. Include empty programs; one through arbitrary input arities; mixed empty/nonempty slots; written versus produced rules; duplicate code; explicit rule consumption; zero, one, and multiple outputs; escaped captures; recursive scopes; and imported libraries. Determine the desired results independently of the current implementation.
2. Choose the availability model and loading grammar. Record the full initial configuration for mixed code/data programs. Decide whether source order has meaning and how textual and structured input express the same placement. Resolve bidirectional syntax separately.
3. Update the source model and lowering together. Today [frontend/source.rs](../frontend/source.rs) has separate `initial` and `rule` fields, and output bodies contain declaration lists. [frontend/lowering.rs](../frontend/lowering.rs) treats grouped values differently from output bodies. A root-only edit would leave two different rule-loading contracts.
4. Establish the authoritative occurrence mechanism in program compilation and state construction, then connect dispatch, rewrite, provenance, capture reachability, equality, and reporting. Remove declaration-based execution only after the replacement implements the chosen visibility contract. Do not keep duplicate authority as an intermediate production behavior.
5. Migrate library loading, fixtures, examples, browser/command inputs, and exact reachability targets in one explicit language revision. [command/main.rs](../command/main.rs) currently accepts declaration-only libraries and concatenates their rules. Automatic wrapping is not a behavior-preserving migration in general. Prefer a clean break with clear diagnostics over a hidden legacy execution mode.
6. Verify the new semantic examples with direct and exhaustive execution, small resume budgets, eviction, competing histories, and resource limits. Compile-time checks and golden snapshots supplement an independent result/provenance oracle; they do not define the semantics. Then rebuild the performance baseline under the new contract.

## Value, cost, and acceptance

The expected value is conceptual consistency and explicit execution authority. A single occurrence lifecycle could simplify dispatch and provenance, but more live rule resources can increase matching, state comparison, capture retention, and history storage. There is no measured speedup and no basis to promise a net reduction in runtime cost.

Planning estimate after semantic decisions: several days for the contract and fixture matrix, roughly one to two weeks for a bounded local-model prototype, and several additional weeks for nested scopes, libraries, provenance, migration, and full verification. These are engineering estimates, not measured effort or commitments. Preserving lexical reach through environment-owned occurrences is a different project and should be estimated after its contract is specified.

Accept implementation only when written and produced occurrences obey the selected contract, consumption has no hidden fallback authority, resource identity survives merge/split correctly, empty-input generalizations are explicit, and exact observations are accounted for. Retain current semantics until that design is approved. This work is separate from the [performance roadmap](roadmap.md).
