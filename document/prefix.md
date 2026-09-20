# Joined prefix reuse

This pass retains multi-input joined prefixes across changes to the remaining input and reduces dispatch bookkeeping. It preserves the frontend, unordered resource semantics, synchronization, capture matching, evidence, and the exact sequence of binding and suspension results. The native Rust kernel is also the browser's WebAssembly kernel.

The baseline is `20984f1`. [Raw measurements and source digests](prefix.json) cover two alternating paired rounds on an Apple M5 Max running macOS ARM64 on AC power, with `caffeinate`. Benchmarks use `bazel run -c opt`; builds complete before paired timings. There are nine timed samples per program in each round, seven per component case, and 25 per exhaustive runtime case. Initialization is measured separately for program cases. This is a measured implementation increment, not completion of the entire [roadmap](roadmap.md).

## Results

Times below pool both rounds' execution samples. Speedups describe these workloads, not arbitrary expressions.

| Workload | Baseline | Current | Interpretation |
| --- | ---: | ---: | --- |
| Repeated join, width 4, 64 suffix changes | 0.070 ms | 0.041 ms | 1.74× |
| Repeated join, width 8, 64 suffix changes | 1.786 ms | 0.518 ms | 3.45× |
| Repeated join, width 12, 64 suffix changes | 43.327 ms | 11.299 ms | 3.83×; individual paired rounds 3.82–4.04× |
| Synchronized program, width 8, 100 transitions | 7.098 ms | 6.369 ms | 1.11× |
| Synchronized program, width 12, 128-way delay, 100 transitions | 226.986 ms | 210.521 ms | 1.08× |
| Prefix changes on every program transition | 7.161 ms | 7.327 ms | 2.3% slower |
| Six factors of two | 70.119 ms | 68.678 ms | 1.02× |
| Decimal `1234567890 + 9876543210` | 201.660 ms | 204.514 ms | 1.4% slower |
| Decimal `12345 × 67890` | 226.597 ms | 227.453 ms | Approximately unchanged |

The component fixture deliberately stresses repeated failed multi-input joins. Width counts candidate `B.C.E` worlds. The prefix combines an eight-token particle, repeated `B` inputs, and an unsatisfied `B.E.E` input; the changing suffix selects `C`. All 64 complete sweeps still expose the same logical work: 5,888, 163,328, and 3,669,248 steps for widths 4, 8, and 12. Caching saves repeated physical matching work while replaying the same suspension stream.

The program fixture adds ordinary synchronized stage rules, runs them through path evaluation, and requires the complete expected final state. Its delay parameter counts `D` inputs in each stage gate; it is not a host sleep. The width-8 case retains exactly 100 events and 105,906 work units; the wide case retains 100 events and 1,651,506 work units. Arithmetic, candidate-domain, and leaf-factor measurements also preserve event and work counts.

Cold and changing workloads remain important. With prefix mutations on every component sweep, pooled execution changes are approximately 5.8% slower at width 4, 2.1% slower at width 8, and 1.9% faster at width 12. The smallest difference is a few microseconds per 64 sweeps. The high-churn full program is slightly slower, as shown above. There is no general arithmetic acceleration claim.

The 19 closed exhaustive-runtime cases retain identical state, event, work, record, and peak counts. Median speedup across cases is approximately 0.993× in each round; individual case ratios range from 0.937× to 1.043×. These timings are close to parity, not evidence of an exhaustive-runtime speedup.

The width-12 component's largest sampled retained count rises from 598 to 602 logical records. Sampling occurs at initialization and after each completed sweep. This is not peak allocator memory, and successful-prefix caches can retain substantially more than this failed-prefix fixture.

## Ownership and reuse

| Owner | Responsibility |
| --- | --- |
| `joining.rs` | Query viability, domain ordering, reuse admission, and final ordinal projection |
| `joining/space.rs` | Candidate domains, delta updates, lazy particle preparation, and leaf-cache accounting |
| `joining/cursor.rs` | Exact traversal with seeded bindings, distinct-world checks, and symmetry constraints |
| `joining/product.rs` | Compose prefix traversal with the remaining input |
| `joining/stream.rs` | Bounded recording, exact replay, frontier continuation, and eviction recovery |
| `dispatch/` | Activate consumers, maintain ordered routing sets, and schedule deliveries |

A query starts with direct traversal. At least two relevant updates must leave its ordered prefix unchanged before the implementation admits a factored traversal. Admission also requires at least three inputs and a wide particle in the prefix. These are generic cost heuristics; they do not recognize arithmetic syntax or values.

The prefix contains all but the final input in the existing cardinality order. The prefix cursor yields a joined binding that seeds the suffix cursor. The suffix uses the same world-distinctness and equivalence-group constraints against that seed. Prefix success and suffix exhaustion each expose the same pending step as the original traversal. Bindings are sorted back into original input positions only at delivery.

Recording begins after reuse of the admitted prefix. Consecutive pending results are represented by a count, and successful prefix bindings retain stable live sites and exact resource identifiers. Reset replays the recorded portion, then continues the existing source cursor from its frontier. It does not require the prefix to have finished before reuse becomes useful.

A suffix-only mutation retains the prefix stream. Any prefix-domain mutation or change to the input order drops the factored traversal and returns to direct traversal. This invalidates removals and recycled sites conservatively. Unchanged relative ordering of surviving prefix worlds preserves the existing symmetry comparisons even when other worlds shift their current ordinals. Captures remain part of the prepared term match.

There is still no runtime enforcement of an execution order in the language. Preserving the implementation's binding and pending stream ensures this optimization also preserves its existing scheduler and budget observations.

## Bounds and eviction

Leaf caches remain limited to 4,096 logical records per query. The new prefix recorder has a separate 4,096-record limit and shares the network's 65,536-record factor budget with leaf caches. The existing whole-query replay cache remains separate.

A prefix recorder also stops after 65,536 logical steps. This bounds the replay position that may need reconstruction during eviction, including when many pending steps compress into one record. Hitting either bound drops recording and continues streaming at the current frontier; it never truncates the answer stream or rejects a program.

Eviction releases the prefix's reserved records and remembers its logical position. The next demand restores that position from the source cursor before delivering another result. At most 65,536 prefix steps require reconstruction. A reset before that demand cancels reconstruction. This is a bound on logical reconstruction steps, not a fixed wall-clock latency bound.

## Dispatch and profiling

Dispatch now uses the existing adaptive ordered set for active plans and scopes: small sets use sorted inline storage, while larger sets use a tree. Affected frame collections use small vectors with explicit sorting and deduplication. Dependency refresh carries arena positions alongside keys, avoiding a second tree lookup. Ascending traversal and consumer order remain unchanged.

In paired instrumented six-factor runs, dispatch takes approximately 39.506 → 38.160 ms, while binding enumeration takes 2.134 → 2.157 ms. These diagnostic timings include instrumentation and are separate from the non-instrumented table above. Dispatch remains the main measured bottleneck. The direct arithmetic path performs no flow composition or proof application in this profile, so optimizing those components cannot explain a speedup in this expression benchmark.

## Verification

Both the debug kernel tests and `bazel test -c opt --nocache_test_results //...` pass. The optimized suite contains 106 targets, including 143 kernel unit tests and browser/WASM conformance. Existing expectations were unchanged.

New differential coverage includes positive and negative joined prefixes, duplicate inputs and resources, captured rule values, partial traversal and repeated reset, relevant suffix updates, prefix invalidation, insertion and deletion, empty domains, cardinality reordering, reused sites, global-budget denial, record saturation, the reconstruction-step bound, and eviction during replay. Retained counts are checked against independent traversal of the retained structures. Ordered-set mutation and copy isolation are compared with a standard ordered set.

Existing deep metaprogramming, runtime production and replacement of rule occurrences, captured execution, recursive support, incompatible histories, and suspension tests remain release gates. The proposed construction of entirely new rule shapes is still a separate language feature; this work neither implements nor restricts that proposal. Passing tests establishes no known regression in this coverage, not a proof that every possible kernel bug is absent.

Native Linux/Windows execution, browser timing, and allocator peak bytes were not measured locally. Browser conformance was executed against WebAssembly.

## Remaining work

Joined results are retained within a query; common joined prefixes are not yet shared across different whole-input plans. Immutable particle plans and identical complete queries retain their existing sharing. Prefix-domain changes still invalidate the prefix rather than retracting individual joined rows.

Cross-plan result sharing needs explicit snapshot ownership, capture-sensitive keys, consumer projection, and shared memory accounting. Broader contextual construction and flow reuse belong to the exhaustive proof path and need their own profiles and evidence-equivalence tests. They remain on the roadmap; this change does not install a graph database, fuse runtime instructions, or parallelize additional kernel mutations.

The measured prefix gain is also bounded by replaying each logical suspension separately. A later batching design could consume recorded pending ranges without issuing one physical call per logical step, but must preserve scheduler interleaving, memory-limit checks, and every supported pause boundary. That requires an explicit runtime protocol and preservation argument before implementation.

## Reproduction

```sh
bazel run -c opt //benchmark:joining
bazel run -c opt //benchmark:prefix -- --width 8 --delay 32 --length 100 --sample 9
bazel run -c opt //benchmark:prefix -- --width 12 --delay 128 --length 100 --sample 9
bazel run -c opt //benchmark:prefix -- --width 8 --delay 32 --length 100 --changing --sample 9
bazel run -c opt //benchmark:expression -- '2*2*2*2*2*2' 2101 --sample 9
bazel run -c opt //benchmark:arithmetic -- 1234567890 add 9876543210 --radix 10 --sample 9
bazel run -c opt //benchmark:arithmetic -- 12345 multiply 67890 --radix 10 --sample 9
bazel run -c opt //benchmark:runtime
bazel test --nocache_test_results //language:test
bazel test -c opt --nocache_test_results //...
```

The expression target takes ternary numerals. The arithmetic target above accepts decimal operands through `--radix 10`. To reproduce the baseline comparison, copy the three new benchmark harness files listed in the JSON into an isolated `20984f1` worktree and register their explicit Bazel targets and measurement module; leave the baseline runtime unchanged.
