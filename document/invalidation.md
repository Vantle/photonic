# Incremental occurrence maintenance

Baseline: `c115754a3a4e056e2b29bdc567b29477355f8b01`, after the runtime occurrence migration. This change preserves that migration's live resource semantics and complete targets. It removes whole-index reconstruction on context changes, directs consumption to selected owners, and reuses the evaluator's reachability result. No program or rule is rewritten.

## Ownership and invalidation

The matching index owns one maintained reverse lexical graph. A transition selects the union of descendants in the old and new graphs, plus contexts that become reachable or retire. Reparenting therefore invalidates both visibility paths. Changes to held evidence alone do not alter lexical visibility. Unaffected world sites, context sites, postings and candidate snapshots survive.

Context invalidation is explicit. Candidate domains, bound plans and dispatch consume the same affected-context set. They do not expand an inherited rule set into thousands of synthetic changed symbols. Surviving locations in an affected context appear in removal and insertion deltas so maintained joins retract and reconstruct their dependent fragments. Actual posting removal precedes site recycling. Location identity stays separate from the current coherence position.

Local context contents are detached and attached only when their particles or reachability change. The index maintains their logical storage count rather than rescanning every context posting during record-budget checks. Consumption groups exact selected resources by owning context and modifies only those frames. The transition evaluator computes reachability once; fingerprint layout and the matching index share the resulting immutable reachability snapshot.

A regression test covers an admission gap previously hidden by broad rebuilds: a required atom first appears in a sibling context, enabling the input globally, and later appears in the context that already contains the other operand. The later local change must admit the missing query even though global availability does not change. Globally changed inputs use the existing availability path; other local changes check for missing subscriptions. Captures and actual executable occurrences still determine consumer authority.

## Independent checks

The new bounded operational oracle implements its own reachability fixed point, lexical traversal, exhaustive operand selection and transition construction. It does not call the production matcher, binding constructor or evaluator. It compares the executable rule read, consumed resources, selected coherences and complete resulting canonical state. Program compilation and state canonicalization remain shared; this is an independent transition oracle, not an independent compiler or proof-projection implementation.

Coverage includes 196 generated programs, fourteen curated occurrence and capture programs, incremental paths with eviction, and nested bodies at depths 1, 2, 3, 8, 16 and 32. Separate index tests compare candidate sets, quantities, postings, storage accounting and stable sites against scans and fresh indexes through mutation, retirement and reuse. Maintained wide rule-sensitive joins are checked through context mutation, partial enumeration and eviction.

All 111 optimized Bazel test targets pass, including 259 runtime tests, the pinned historical comparison, native/WebAssembly conformance and headless Chrome. Nine complete command reports match the baseline after removing only the top-level work count. State, event, occurrence, capture and provenance fields compare structurally. The recorded arithmetic examples change only their verified work counts.

## Measurement

The raw audit in [invalidation.json](invalidation.json) records sequential optimized native runs on the development Apple M5 Max, executable and source hashes, benchmark arguments, fixture sources, cold observations, every measured sample, allocation diagnostics and rejected experiments. Timings run without concurrent builds or tests, on AC power. Allocation instrumentation runs separately. Requested heap bytes are not RSS or WebAssembly heap usage.

The initial matrix exposed redundant admission work in the availability control. Its raw failures are retained. A follow-up separates globally changed inputs from local admission and increases sample counts for the small lifecycle controls. Acceptance uses the existing 5% tolerance above one millisecond and 10% below it; no thresholds were widened.

The final paired matrix uses three alternating rounds: 101 samples per small direct lifecycle, 21 per wide consumption, expression and availability control, and three per full export. Longer follow-ups use 101 samples per wide and small availability control, and 51 per 4,096-rule availability control. The scope control includes diagnostic phase instrumentation in both revisions; complete expression and lifecycle timing use production builds.

| Workload | Baseline | Current | Ratio |
| --- | ---: | ---: | ---: |
| 64 ordinary transitions | 0.135 ms | 0.135 ms | unchanged |
| 32 body entries and returns | 0.533 ms | 0.198 ms | 2.70× faster |
| 32 context consumptions | 0.259 ms | 0.234 ms | 1.11× faster |
| 256 staged context consumptions, follow-up | 15.48 ms | 12.11 ms | 1.28× faster |
| `2+2` | 130.69 ms | 10.59 ms | 12.34× faster |
| `2*2*2*2*2*2` | 1,945.14 ms | 91.71 ms | 21.21× faster |
| `(2+2)*(2+2)` | 629.16 ms | 35.64 ms | 17.65× faster |
| Six-factor complete lifecycle with streaming export | 4,753.58 ms | 2,867.04 ms | 1.66× faster |

The availability controls remain within the execution tolerance after the longer follow-up: 64 rules/64 contexts take 1.928 versus 1.969 ms, 256/16 take 8.545 versus 8.686 ms, and 4,096/1 take 98.841 versus 99.520 ms. Full lifecycle performance remains within tolerance or improves.

Two isolated phase regressions remain visible. Initialization of the 4,096-rule control increases from 1.251 to 1.322 ms, about 5.7%. Reporting in the wide-consumption fixture increases from 13.569 to 14.746 ms, about 8.7%; its complete lifecycle still improves from 33.259 to 31.120 ms. Reporting code is unchanged, and the cause of that phase regression has not been isolated. These results do not establish that every phase passes its individual tolerance. Neither failure was removed from the audit or absorbed by widening the gate.

Execution allocation traffic falls from 3.47 GB to 102.43 MB for the six-factor case, and from 44.53 to 9.42 MB for wide consumption. Wide retained engine storage falls from 12.62 to 11.37 MB. Complete six-factor streaming-export peak is essentially unchanged at 439.45 versus 439.48 MB: reporting remains a substantial separate cost. Ordinary peak increases about 2 KB, and the 32-consumption fixture about 6 KB. Every diagnostic releases tracked allocations to zero.

A separate instrumented six-factor profile attributes about 15.7 ms to indexing, 50.4 ms to dispatch, 9.2 ms to rewriting and 8.7 ms to fingerprint maintenance. Nested phase times overlap. Explicit context invalidation lowered index time from approximately 102 ms in an intermediate version to 16 ms. These profile measurements explain the change but do not replace the production timing matrix.

## Rejected experiments

Replacing live context particles with the existing persistent sequence reduced retained memory about 15% and execution allocation traffic about 21% in the 256-consumption fixture, but regressed execution about 7% there and 13% in the expression fixture. That prototype was removed. Sharing must account for traversal and identity lookup, not just copying.

Two subscription-membership prototypes separated lightweight consumers from optional boxed search state. Neither maintained candidate domains while disabled. Eager installation of every executable membership improved availability churn but increased six-factor execution from about 89 to 524 ms. Retaining only previously demanded memberships still increased it from about 92 to 176 ms. Both were removed. Repeated consumer reconciliation under context changes outweighed the construction savings; avoiding inactive searches alone is insufficient. The raw trial results and the demand prototype patch are retained in the audit.

## Remaining boundary

Incremental context indexing and owner-directed consumption are delivered. Context particles still use vectors. Identity hashing still has broader reconstruction paths on frame changes. Subscription membership still follows the existing availability and request lifecycle. A future implementation must update occurrence memberships from owner deltas without repeatedly constructing complete consumer lists, and must beat these measured controls.

The subsequent [exhaustive replay audit](transcript.md) corrects a recycled-site dependency gap and evaluates a rule-sensitive extension. The extension was removed after matcher reuse failed to produce complete-program gains. Completed exhaustive replay therefore still excludes rule-sensitive queries. Safe dependencies and stream checks are necessary, but measured runtime benefit is also required. Arbitrary shared interior joins and lazy trace composition remain conditional scaling work. No universal runtime rating or optimality claim follows from this benchmark suite.

The later [population audit](population.md) replaces context vectors with immutable populations and membership bitmaps, and replaces repeated fingerprint dependency discovery with a maintained graph. It substantially improves wide consumption, but records an unresolved small-scope phase regression and increased arithmetic allocation traffic. Its measurements and validation supersede the storage and fingerprint boundary above without changing this audit's historical results.

```sh
bazel test -c opt //... //toolchain/browser:check
bazel run -c opt //:format -- --check
bazel run -c opt //benchmark:expression -- '2*2*2*2*2*2' 2101 --sample 21
bazel run -c opt //benchmark:scope -- --width 256 --depth 16 --length 64 --sample 21
bazel run -c opt //benchmark:allocation -- expression '2*2*2*2*2*2' 2101 --sample 1 --export view --writer
```
