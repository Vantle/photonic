# Compiled scope membership

Baseline: `4b38365`. Declaration lookup and live availability now use the same fixed positions within each compiled scope. This replaces a hash table of separately allocated consumer lists and a mutable ordered set of global input identifiers with flat immutable declaration storage and compact availability masks. Original rules, programs, captures, delivery order and logical accounting are unchanged.

## Ownership and representation

[Scope](../language/catalog/scope.rs) owns one flat rule-index array and a sorted array of input groups referring to ranges within it. Stable grouping preserves the order of consumers with equal inputs. [Catalog](../language/catalog.rs) records each input's declaring scope and its position there. The rule program remains unchanged; sorting applies only to the dispatch lookup metadata.

An availability change uses those precompiled locations directly. It no longer searches and edits an ordered set of global input identifiers in every declaring scope. Consumer discovery enumerates active local positions and reads their rule ranges directly, avoiding a hash lookup for each group. When a selected-input change set is smaller than the active scope, discovery maps those selected global inputs to local positions by binary search. Both paths emit the same ordered requests.

The [bitmap container](../language/bitmap.rs) keeps scopes of up to 64 groups in one word. Wider scopes use an array of words and another bitmap describing which words are occupied. Each additional level reduces the universe by a factor of 64. Updates propagate only when a word becomes empty or nonempty. Iteration skips inactive words and does not allocate. This bounds hierarchy depth by the identifier width and avoids scanning all declarations when a wide scope has few active inputs.

The bitmap is a fixed-capacity index for compiled local positions. It does not replace the general membership container used for unbounded identifiers elsewhere. Storage is allocated during scope construction; ordinary availability changes do not allocate new tree nodes. A live scope mask and the global enabled-input set remain synchronized through the existing coherent availability update, which processes removals before insertions.

Logical retained counts continue to charge the same input groups, consumer identifiers, owner relationships and enabled memberships. A richer compiled owner location still denotes one relationship. Bitmap capacity, like prior ordered-set capacity, does not alter logical membership counts. The regression trace checks exact public accounting through mutation and eviction. This work does not retain removed searches or cache their results, and it does not reduce subscription admission counts.

Empty input slots remain initially enabled. `[]`, `[,]`, higher arities, empty outputs and mixed inputs retain their existing behavior. Distinct input slots still select distinct coherences; an empty particle requirement is not the same as an absent coherence.

## Scaling benchmark

The new [scope benchmark](../benchmark/scope.rs) measures the representation's intended workload separately from complete arithmetic execution. It constructs a fixed program with varying declaration width and number of scopes, then repeatedly enables and disables selected inputs through coherent resource changes. The dispatcher fully drains each generation. Every delivery contributes its rule, frame, owner and selection to a checksum, alongside work, delivery, preparation, reuse and retained-record counts.

Initialization measures index and dispatcher construction. Execution includes changing state, updating the index and dispatcher, and consuming the resulting searches. Release drops the index and dispatcher. Fixture parsing, program construction and fixture destruction are outside these timings. The total is therefore an index/dispatch lifecycle, not complete program execution or process latency. These are synthetic scaling results, not arithmetic speedup estimates.

The same benchmark implementation is compiled against the baseline and candidate. Three rounds alternate their order; each process warms once and records five fresh instances. All non-timing observations must agree. The existing complete-program, allocation and exact-report controls remain independent acceptance requirements.

## Measurement and verification

The [raw audit](scope.json) records baseline and candidate hashes, all samples, allocation diagnostics, profile attribution, exact comparisons, fixtures, rejected prototypes and reproduction scripts. Measurements run serially on the current Apple M5 Max after builds and tests complete. Existing acceptance tolerances remain 5%, or 10% below one millisecond.

| Fixture | Inputs per scope | Scope count | Input stride | Baseline lifecycle | Candidate lifecycle | Speed ratio |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| tiny | 8 | 1 | 1 | 0.245 ms | 0.242 ms | 1.01× |
| single | 64 | 1 | 1 | 2.334 ms | 2.072 ms | 1.13× |
| fanout | 64 | 64 | 1 | 4.700 ms | 2.269 ms | 2.07× |
| dense | 256 | 128 | 1 | 33.813 ms | 11.982 ms | 2.82× |
| sparse | 4096 | 64 | 4096 | 16.295 ms | 3.023 ms | 5.39× |
| separated | 4096 | 64 | 64 | 20.734 ms | 5.548 ms | 3.74× |
| many | 16 | 1024 | 1 | 3.633 ms | 1.276 ms | 2.85× |
| wide | 16384 | 1 | 4096 | 5.641 ms | 4.850 ms | 1.16× |

All eight total-lifecycle comparisons pass. Initialization and update execution also pass for every fixture. The dispersed sparse case now improves update execution from 4.37 to 2.60 ms, resolving the earlier prototype's regression. The tiny fixture's initial release medians were 1.71 versus 1.96 microseconds, outside the submillisecond tolerance. Three alternating processes per side with 501 samples each measure 1.04 versus 0.96 microseconds; both datasets are retained. This calibration changes sample volume, not the tolerance.

The existing 42 protected comparisons pass after one follow-up. The original changing-prefix comparison measured 3.83 versus 4.04 ms, about 5.4% slower. Three alternating rounds with 101 samples per process measure 3.80 versus 3.98 ms, about 4.6% slower and within the same 5% gate. This remains a regression in that workload, despite passing the gate. The stable-prefix control is about 3.8% slower. The other 41 comparisons pass their original samples. No non-timing observation changes.

Compact six-factor arithmetic measures 61.30 versus 60.38 ms; decimal addition 182.86 versus 183.34 ms; decimal multiplication 201.89 versus 202.97 ms. These are approximately flat, and do not support describing the synthetic scope gains as a large complete-program execution speedup. Six-factor initialization allocation calls fall from 105,191 to 104,306. Requested initialization traffic falls about 31 KB. Execution allocation traffic and full-export peak are approximately unchanged. Every allocation diagnostic returns to zero additional tracked retained bytes after release. Requested bytes are not process RSS or WebAssembly heap measurements.

A separate profile still records 66,492 search admissions. Availability maintenance measures about 5.33 versus 5.02 ms and request discovery about 2.78 versus 2.58 ms. Instrumented total execution is flat at about 66.52 ms; nested scopes overlap. The larger admission/removal lifecycle remains open.

All 110 optimized Bazel test targets pass, including native/WebAssembly conformance. The kernel contains 230 passing tests. The bitmap oracle compares every mutation and ordered traversal with an ordinary ordered set across inline and hierarchical boundaries, including removal to empty and the highest valid position. The catalog oracle checks stable grouping with duplicate and reversed rule occurrences. Existing dispatcher mutation tests now independently reconcile every scope mask with global availability and compiled declarations. Benchmark tests verify expected delivery counts and stable observations through alternating activation.

Twenty-two complete and paused command reports and sixty empty-input reports compare byte-for-byte with the baseline. The complete 128-step dispatcher trace agrees, including captures, resource identity, consumer order and logical accounting. The final measured executable hashes match the final production builds.

```sh
bazel test -c opt //...
bazel run -c opt //:format -- --check
bazel run -c opt //benchmark:scope -- --width 256 --depth 128 --length 64 --stride 1 --sample 5
bazel run -c opt //benchmark:scope -- --width 4096 --depth 64 --length 64 --stride 64 --sample 5
```

## Rejected forms

A direct intersection of immutable scope declarations with global availability removed the per-scope sets, but preliminary arithmetic results were mixed and it could scan many disabled declarations. That form was removed.

The first compact-mask prototype used the existing ordered membership set to index occupied words. It improved setup and total lifecycle time but regressed the separated sparse-update case about 8%. Replacing that secondary ordered set with the bounded bitmap hierarchy removes the regression. Both prototypes and their raw measurements are retained in the audit.

## Remaining architecture

This delivers compiled scope membership and improves its scaling under availability changes. Subscription discovery still constructs requested consumer lists; reconciliation still removes and admits searches. Coherent subscription lifecycle maintenance remains the main execution opportunity. Any larger reuse mechanism must establish exact occurrence, context, evidence and accounting boundaries and justify its maintenance cost with complete-program benefit.
