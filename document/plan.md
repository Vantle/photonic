# Shared matching structure

Baseline: `40ef9f7`, whose production code is identical to `bdc4de1`. This implements the first representation slice of the [architectural assessment](opportunity.md): separate shared input structure from bound context and live subscription state. Rules, programs, execution order and public interfaces are unchanged.

## Ownership and behavior

[Input](../language/plan.rs) owns one immutable shape containing the original ordered symbols, compiled particle fragments and symmetry groups. A [context](../language/plan/context.rs) holds one shared shape handle and an owner parameter. Creating a captured context no longer allocates a complete matrix of bound terms. Atom terms discard the owner as before; executable-rule terms retain the exact owner.

[Space](../language/joining/space.rs) owns the context separately from its mutable candidate domains, particle cursors, subscriptions and retention state. It no longer owns a second bound-pattern matrix or a separate symmetry-group handle. Immutable query structure and mutable occurrence state have distinct lifetimes. No dormant search cache, invalidation policy, threshold or background maintenance is introduced.

The shared shape retains original symbol order and multiplicity for keys and reporting position mappings, while compiled fragments group repeated symbols for matching. Symmetry classification on the unbound symbols gives the same classes as classification on terms with one common owner. Empty rule input retains its existing normalized empty particle. Input arity and logical retained-record charges remain unchanged.

Bound terms are produced on demand at key boundaries. Candidate keys collect an iterator of terms, then apply their existing sorting and deduplication. Joined-prefix keys retain every ordered term and its exact capture. Thus allocation sharing does not merge live occurrences, remove multiplicity, weaken capture identity or change what can be reused. Candidate membership, search progress, executable reads and delivery remain private to their existing owners.

The [query boundary](../language/joining/query.rs) has one production variant containing a planned context. Its raw-pattern variant is compiled only for tests, preserving the independent matcher used by differential tests, including arbitrary per-term captures. The old optional raw-pattern production path is removed. The new context module and query module are listed explicitly in Bazel; no dependencies, unsafe code or platform-specific implementation are added.

## Measurement

The [raw audit](plan.json) records source and executable hashes, machine and power information, paired samples, separate allocation diagnostics, separate profiles, fixtures and reproduction scripts. Baseline executable hashes match the accepted registry audit. Optimized executables run serially on the current Apple M5 Max on AC power after builds and tests complete.

Three rounds alternate baseline and candidate. Compact execution uses nine samples per process; lifecycle measurement records a cold run separately and five fresh-instance samples. Allocation counters and instrumented profiles do not supply timing acceptance. The additional maintenance and symmetry controls retain their existing calibration and tolerances.

| Complete workload | Baseline | Candidate |
| --- | ---: | ---: |
| Compact six-factor execution | 63.590 ms | 61.492 ms |
| Ten-digit decimal addition | 187.544 ms | 185.084 ms |
| Five-digit decimal multiplication | 214.311 ms | 207.301 ms |
| Six-factor full lifecycle | 643.342 ms | 636.229 ms |

Compact execution improves approximately 1.3–3.3% across these arithmetic cases. Full lifecycle improves approximately 1.1%. Preliminary arithmetic medians improve approximately 1.4–3.3%; neither set establishes a universal speedup. This is a modest execution gain from a simpler representation, not the larger lifecycle optimization proposed in the assessment.

| Six-factor execution allocation | Baseline median | Candidate median |
| --- | ---: | ---: |
| Allocation calls | 661,282 | 542,949 |
| Requested bytes | 88,521,447 | 81,769,511 |

The change removes about 118,000 allocation calls, a 17.9% reduction, and 6.75 MB of allocation traffic, a 7.6% reduction. Full-export peak remains approximately 372 MB; retained engine storage falls only about 0.13 MB. Canonical history, the owned report and the encoded buffer still dominate that peak. Every allocation diagnostic returns to zero additional retained bytes after release. Requested-byte counters exclude allocator metadata, stack storage and process RSS.

The separate paired profile measures dispatch at about 39.05 versus 37.02 ms and subscription work at about 25.82 versus 23.84 ms. Preparation changes from 9.63 to 8.99 ms, admission inclusive of preparation from 14.19 to 13.49 ms, and removal from 5.81 to 4.70 ms. These are nested instrumented scopes and cannot be summed. Search admissions remain 66,492 and firings remain 10,101; the change reduces ownership and construction overhead per admission rather than eliminating admissions. All profiled non-timing observations agree.

All 42 protected comparisons pass at the existing 5% regression tolerance, or 10% below one millisecond. They cover complete export, compact execution, inspection, symmetry, shared queries, preparation, large candidate domains, prefix churn and dispatch at several frame counts. The 64-level full lifecycle is about 4.1% slower within the gate; the data does not support claiming every workload is faster.

## Verification and next boundary

All 109 optimized Bazel test targets pass, with 105 freshly executed and four cached, including native/WebAssembly conformance. The kernel contains 219 passing tests. A new ownership test drops the input while two contexts remain, checks ordered duplicate terms and distinct captures through the maximum owner identifier, and verifies that the shape and its fragment are released after the last context is dropped.

Existing differential tests compare planned matching against the raw-pattern path through changes, preparation rejection, pause/reset, new consumers, eviction, captures and resource pressure. Metaprogram tests cover nested rule execution and matching, capture isolation, competing evidence and resumed execution. Sixteen complete and paused command reports compare byte-for-byte with the baseline, and the 128-step dispatch verification trace agrees without filtering fields. Formatting and build-integrated checks pass.

Logical storage accounting intentionally keeps its existing charges: reducing physical allocation must not silently change bounded execution behavior. No scheduling or rule semantics changed, so exact historical comparisons remain appropriate here.

Subscription admission and removal remain substantial. Further work needs to distinguish necessary fresh occurrences from avoidable consumer rediscovery or domain reconstruction under coherent changes. Repeated frame/input/owner coordinates alone still do not establish cache validity. The subsequent [report-view implementation](view.md) removes complete owned-report materialization from the three JSON report consumers and reduces full-export peak memory.

```sh
bazel test -c opt //...
bazel build -c opt //benchmark/... //command:photonic
bazel run -c opt //:format -- --check
bazel run -c opt //benchmark:expression -- '2*2*2*2*2*2' 2101 --sample 9
bazel run -c opt //benchmark:allocation -- expression '2*2*2*2*2*2' 2101 --sample 3
bazel run -c opt //benchmark:dispatch -- --verify
```
