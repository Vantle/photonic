# Runtime rule occurrences

Contract authorized September 22, 2026. This supersedes the alternatives in the [loading assessment](loading.md). The contract sections below are current; the [migration record](#migration-record) describes the change as it landed. Loading, executable availability, ownership, and exact targets have migrated to live rule occurrences. That migration left the grammar unchanged. The runtime preserves rule definitions; it does not construct replacement rules or specialize their outputs.

## Values and ownership

Immutable code and live resources have separate identities. Loading introduces one root context occurrence for every rule the program defines, including repeated equal definitions. Entering an output body introduces that body's rules with captures of the new context. Neither operation introduces atoms mentioned inside rule inputs or outputs. Written and dynamically emitted rules share code interning and occurrence construction. Interning uses normalized definitions as keys while compiling the original definition.

Every live occurrence belongs to a coherence or a lexical context. Context members are visible in lexical descendants. Visibility exposes the original occurrence; it does not copy it. Held operands are retained evidence, separate from live members. A compiled catalog supplies immutable matching plans and initialization metadata, never executable authority.

Executing a rule reads its live occurrence. Explicit input selection consumes operands, including rule values. If an occurrence is both read and consumed, consumption wins. Removing a context-owned occurrence removes every lexical access path to it in the resulting state. Historical states and independent execution branches are immutable and remain intact.

Consumption is per owning location. If coherences share resource X, selecting X in one coherence does not consume X in another. This also applies when both coherences participate in the same application: an unselected X survives into the merged remainder. Unconsumed shared identities coalesce when their owners merge; independently introduced equal values remain distinct. Provenance records the surviving occurrence's actual owner. Context members stay in their context rather than joining coherence remainders.

Contexts remain reachable through active coherences, lexical ownership, and captures. A context retires when those references disappear. Retaining immutable code cannot retain executable availability after occurrence consumption or context retirement.

## Matching locations and startup

Nonempty input positions can bind a coherence or a lexical context. Each reachable context supplies one matching location. Rule-valued operands can therefore match without unrelated data or an artificial coherence. A coherence exposes its local values plus visible context members. Bindings retain each operand's actual ownership location, even when it is accessed through another context or coherence.

Distinct input positions require distinct locations. Multiple operands inside one particle can select distinct occurrences in the same location. Multiple visibility paths to one context occurrence cannot satisfy repeated operands. Sharing an identity across different coherence owners supplies distinct occurrences under the consumption contract.

| Input | Contract |
| --- | --- |
| `[] A` | Zero input positions. The loaded rule's owning execution context is sufficient to produce A. |
| `[()] A` | One empty particle pattern. Selects an actual coherence and preserves its unmatched members. |
| `[(), ()] A` | Two empty particle patterns. Requires two distinct actual coherences. |
| `[A,()] B` | One A-containing location and one distinct actual coherence. |
| `[[A] B] C` | Selects a matching rule occurrence from a context or coherence and consumes it. |

These rules generalize to arbitrary arity within explicit resource limits. Zero-input execution does not manufacture an input coherence or impose a one-shot restriction. A context-owned zero-input rule executes at its owning context, not once per visibility path. A coherence-owned zero-input rule reads its occurrence without implicitly selecting the containing coherence as an operand.

Brackets that share a term are separate sources. Each consumes what it names and produces every other part of the term, one rule per part: `[A] [B]` loads `[A] B` and `[B] A`, so A and B become each other, and `[A] [B] C` adds `[A] C` and `[B] C`. Lowering writes these rules out directly. The frontend budget bounds everything it writes, rule names included, to 1,000,000 units plus eight per byte of source, because a rule's name repeats the text of every rule nested inside it. Report notation such as `⟨…⟩`, `§0`, and `@f0` is diagnostic labeling, not additional source grammar; capture and ownership are explicit structured report fields.

## One evaluator

`language/evaluation.rs` implements the state transition used by both direct traversal and exhaustive exploration: input consumption, remainder, output introduction, body entry, held evidence, reachability, and reclamation. The duplicated `rewrite.rs` and `recipe.rs` implementations have been deleted.

The two search strategies serve different queries. Direct traversal follows one execution path and can witness reachability. Exhaustive exploration also discovers applications through supported derived views, projects bindings to concrete sources, imports captured environments, and maintains proof support. Its application adapter prepares those inputs and records provenance around the same transition kernel. Direct failure remains Unknown; a closed exhaustive search can establish Unreachable. Neither strategy independently defines consumption or output semantics.

Dispatch resolves indexed eligible plans to live occurrences. Lexically nearer consumers retain scheduling priority within shared input groups. Index updates preserve resource ownership, capture, and reachability dependencies. Mixed atom/rule candidate updates account for unchanged visible context operands. The subsequent [incremental occurrence maintenance](invalidation.md) preserves unaffected index state and explicitly invalidates changed lexical contexts. Completed rule-sensitive exhaustive transcripts are not reused across snapshots; that extension remains a separate equivalence and measurement obligation.

## Exact targets

Targets specify the complete state, including live rules. `A, [A] B` reaches `B, [A] B`; it does not reach bare `B`. After `[A] B, [[A] B] C` consumes the first occurrence, the complete target is `C, [[A] B] C`. Target compilation resolves rules and atoms by lookup, copies the program only for a target that mentions a symbol the program never interned, and never alters execution.

Textual targets describe root coherences and independently introduced root rule occurrences. They cannot yet encode arbitrary shared occurrence graphs or captured nested contexts. Canonical equality still accounts for those structures; this limitation concerns expressing a target, not ignoring parts of state.

Every `photonic_test` target also includes the loaded root rules, as Spectrum's exact claims and path goals do with `--preserve`. Existing data fixtures remain useful for arithmetic and protocol assertions, but are not implicitly complete runtime targets; `prism` reads its target file as the complete configuration. Adding the root rules copies unchanged definitions; it does not infer which rules should survive. Consuming programs must specify their surviving rule occurrences themselves.

## Migration record

The `path::Search::new` and `prism::Search::new` constructors now return searches directly; target declarations are valid and the old declaration rejection error is removed. `prism::Search::target` no longer returns that error. Report targets contain the full source program shape rather than only initial particles.

The arithmetic runner exports `program.wave` and complete `target.json` files. The webbook's command examples construct explicit targets, and its live queries include surviving rules. Recorded inspection views include context particles and resolve immutable rule text through the report's `definition` catalog. Context occurrence references serialize identity, label, and capture without repeating large rule bodies in every state. Owned Rust token views retain their display text.

The one source fixture formerly using `[]` to select an existing empty particle now explicitly uses `[()]`. Legacy reference changes are narrow: the shared-resource merge case gains exactly one supported state and two events required by consumption per selected occurrence. Interned instruction labels and body scope labels move, and preserved original rule names replace prior normalized placeholders. Unrelated expected states were not accepted through wholesale snapshot replacement.

### Validation

The final `bazel test -c opt //... //book:check` run passes all 111 test targets, including native execution, WebAssembly, frontend conformance, programs, arithmetic, libraries, the pinned-runtime comparison, and headless Chrome. The runtime suite contains 249 passing tests. Interface checks cover explicit target export, context inspection, and live complete-state WebAssembly queries. `bazel run -c opt //:format -- --check` passes.

Regression coverage includes zero-input startup and repetition; empty and mixed input arities; loaded and emitted rule matching; equal code with distinct occurrence and capture identities; lexical visibility without duplication; consumption, read-plus-consume, and merge remainders; surviving provenance and immutable history; captured environment import; context retirement; incremental matching versus rebuilt search; eviction and recycled storage; tight limits, interruption, and resumption. Exact-target cases run through direct and exhaustive searches.

The differential oracle compares 540 finite atom programs against the pinned older evaluator. Those programs deliberately exclude changed loading observations, context operands, and capture semantics; the comparison checks the unchanged data behavior, supported states, and events within that domain. It is not an independent oracle for the newly authorized semantics. Dedicated occurrence regressions establish those contracts.

The expression record retains its arithmetic results and 10,017 events, with work changing from 24,690 to 24,768. Smaller expression records retain their results and event counts. Six native graph records retain their state/event counts. The four circuit records retain their arithmetic results and event counts; division has a permitted event-order change with the same multiset of rule applications. The stream record retains its eleven acknowledgements/events. Older circuit and graph work counts predate intervening optimizations and are not used as migration speedup evidence.

### Measurements and remaining architecture work

[Raw measurements](occurrence.json) compare the former declaration model at `ec6d707` against live occurrences on the development Apple M5 Max. Native optimized binaries run sequentially. Ordinary and scope cases use six allocation-instrumented samples in alternating paired batches; availability, inert-rule, and consumption cases use five samples. Medians below are descriptive measurements, not cross-machine guarantees. Allocation instrumentation affects timings. Requested allocation bytes are not resident memory.

| Workload | Previous execution | Current execution | Current / previous |
| --- | ---: | ---: | ---: |
| 64 ordinary transitions | 79.2 µs | 140.4 µs | 1.77× |
| 32 body entries and returns | 111.1 µs | 566.3 µs | 5.10× |
| 256 rule codes, 16 contexts, 64 availability updates | 8.73 ms | 9.45 ms | 1.08× |
| 1,024 inactive rules and 128 transitions | 190.5 µs | 1,231.0 µs | 6.46× |

Ordinary initialization changes from 64.2 to 107.8 µs and release from 28.0 to 53.5 µs; peak requested storage changes from 319,194 to 956,459 bytes. Scope initialization changes from 68.1 to 112.5 µs and release from 28.9 to 58.8 µs; peak requested storage changes from 329,995 to 705,059 bytes. Thirty-two context-rule consumptions take 264.0 µs after 464.9 µs initialization, with 102.0 µs release and 1,274,680 peak requested bytes. That new operation has no equivalent in the previous runtime. Every allocation-instrumented run releases its tracked allocations to zero.

The additional resources and complete targets do observable work the declaration model did not perform. These costs are real; this migration is not a performance improvement or a claim of superoptimization. The highest-value follow-up is incremental context membership and reachability maintenance, followed by persistent context occurrence storage and lower-cost identity/capture accounting for large rule sets. Measure initialization, traversal, churn, memory, and release together. Preserve occurrence-level invalidation and one transition kernel; do not regain speed by treating consumed rules as executable declarations or changing rule contents.
