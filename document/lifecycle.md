# Runtime lifecycle audit

Runtime baseline: `3845189`. This increment adds benchmark instrumentation and regression checks. It changes no production evaluator, program, rule or public runtime interface. Measurements target the current Apple M5 Max on AC power, using optimized Bazel builds on macOS 26.6.2.

## Boundaries

`//benchmark:lifecycle` measures five separate phases: initialization, execution, full reporting, compact JSON serialization and release. Initialization includes cloning the parsed input and constructing a fresh engine. File loading, parsing and fixture generation precede measurement. Reporting calls the existing direct-path `report()` or exhaustive `snapshot()` interface. Serialization retains the resulting JSON buffer until release. Release drops the engine, report and buffer together. The total is the sum of these five durations; checking the output fingerprint and printing benchmark results are excluded.

`//benchmark:allocation` runs the same lifecycle with a benchmark-only global allocator that delegates to Rust's `System`. Its timing includes atomic accounting overhead and is diagnostic only. Production binaries and the ordinary lifecycle timing binary do not contain this allocator. No dependency or production feature was added.

Each process reports its first evaluation separately as `cold`, followed by five fresh-instance samples. These later samples have warmed process and allocator state, but do not reuse an engine or its caches. The [raw audit](lifecycle.json) records three rounds with alternating executable order, all cold and subsequent samples, arguments, fixture source, source and executable hashes, baseline revision and power state. Builds, tests and profiling finished before these timed runs. There is no before/after runtime comparison or speedup claim in this audit.

## Findings

The table uses medians of fifteen subsequent samples per workload from the uninstrumented executable. Millisecond values are rounded, and a median of totals need not equal the sum of phase medians.

| Workload | Initialization | Execution | Full reporting | Serialization | Release | Total |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Ternary `2+2` | 1.953 | 6.285 | 26.534 | 4.623 | 2.813 | 42.221 |
| Three factors of two | 2.024 | 17.928 | 101.240 | 16.080 | 8.423 | 145.663 |
| Six factors of two | 2.312 | 62.858 | 597.391 | 81.952 | 37.974 | 782.961 |
| Ternary `(2+2)*(2+2)` | 2.138 | 23.676 | 165.560 | 24.991 | 12.339 | 231.066 |
| 128-step direct chain | 0.118 | 0.149 | 0.151 | 0.033 | 0.060 | 0.517 |
| Exhaustive diamond | 0.003 | 0.026 | 0.004 | 0.003 | 0.004 | 0.039 |

Two additional insufficient-capacity controls cover exhaustive token and coherence rejection. All eight workloads preserve the same state/event/work counts, serialized byte length and report fingerprint across executable variants, rounds and samples. A fingerprint comparison is a diagnostic, not a substitute for exact semantic tests.

The six-factor report contains 10,102 states and 10,101 events and serializes to 125,450,547 bytes. Its allocation diagnostic records approximately 2.475 GB of requested allocation traffic and 515.8 MB of peak live requested storage above the parsed-input baseline. Reporting alone accounts for 2.246 GB of traffic, 7,775,817 successful allocations and 2,017,541 successful reallocations. Execution accounts for about 89.9 MB of traffic and 661,302 allocations. Small variations in execution allocation counts and capacity appear across fresh processes; the raw samples retain them. Every measured lifecycle returns to zero additional tracked retained bytes after release. This is evidence for these serial workloads, not a general leak-freedom claim.

A separate five-second, one-millisecond stack sample of the six-factor lifecycle identified `path::Search::report`, lazy `Record::canonical` initialization, refinement/partitioning and state renaming as major reporting costs. This agrees with the source: full reporting requests a canonical node for every retained state. Profiling was exploratory and is excluded from the timing table.

The normal direct-path command already calls `summary()` for text output. It therefore does not pay for the full retained-history report measured here. Full-report optimization has value for that consumer, while dispatch and index maintenance remain relevant to compact execution. The benchmark also retains a JSON buffer; a command that streams JSON directly to stdout has a different peak-storage profile. The small exhaustive controls establish coverage, not a representative bound on large exhaustive proof workloads.

## Interpretation and next work

The runtime has substantial tested optimization, but these measurements do not justify calling it globally optimized. The execution timer was answering a narrower question than complete history export. Reporting is now an independently measurable optimization target.

For the six-factor full lifecycle, halving the roughly 597 ms reporting phase while holding everything else fixed would reduce approximately 783 ms to 484 ms, about 1.62× faster. Halving the roughly 63 ms execution phase alone would yield about 1.04× on this full-report workload. These are arithmetic scenarios, not forecasts; compact execution has a different denominator. Approximately 91% of requested allocation traffic in this lifecycle occurs during reporting. Neither traffic nor retained-byte count determines attainable latency improvement by itself.

The next reporting experiment should reduce repeated canonicalization/refinement allocation or temporary storage while preserving every canonical state, resource mapping, event and ordering. Keep changes only if full export improves and compact execution remains within its existing gates. A narrower execution experiment should first attribute the measured allocation traffic to dispatch/index maintenance. More caching, parallelism or new abstractions require evidence that they save more than their maintenance cost. Novel rule construction and rewriting remain excluded.

## Allocation contract and safety

The diagnostic follows Rust's [GlobalAlloc contract](https://doc.rust-lang.org/std/alloc/trait.GlobalAlloc.html). Every unsafe callback forwards the caller's pointer, layout and size directly to `System`; it neither dereferences nor manufactures a pointer. Successful allocation and zeroed allocation add the requested layout size. Successful reallocation adds only positive growth or records released shrinkage; failure leaves accounting and the original allocation unchanged. Deallocation records the original layout size and delegates the free. Callbacks perform only integer comparisons and non-allocating atomic operations, with wrapping arithmetic where needed; they do not format, allocate recursively, assert or unwind. The production language and plain timing executable continue to forbid unsafe code.

`allocation` counts successful ordinary and zeroed allocations; `reallocation` counts successful resizes, including same-size resizes; `deallocation` counts frees. `allocated` sums initial requested bytes and positive resize growth. `released` sums shrinkage and frees. They do not count every byte copied by a reallocating allocator. `retained` is the signed live-byte change during the phase. `peak` is the largest additional live requested storage above that phase's entry baseline. The aggregate peak accounts for live storage carried into later phases.

Only serial evaluation with quiescent phase boundaries is supported for these measurements. Atomic counters keep callbacks race-free, but snapshots and peak resets are not a transaction across threads. Native allocator metadata, temporary storage inside `System`, stack memory, process RSS, WebAssembly memory and allocations outside the measured intervals are excluded. Optimizer allocation elision is permitted by the Rust contract; these counters describe this executable, not semantic allocation requirements. Counter deltas use wrapping subtraction; samples must not span a complete counter wrap. Nested or overlapping measurement scopes fail explicitly, and a guard restores availability during unwinding.

## Verification and reproduction

Allocator tests check alignment, zero initialization, content preservation through growth and shrinkage, overlapping allocation lifetimes, balanced release, and recovery after nested or panicking measurement scopes. The integration test runs both executables through direct, exhaustive and expression evaluation, compares observations, checks accounting and complete release, and rejects zero sample/budget values, unfinished runs, unreachable direct targets and invalid expression digits.

All 109 optimized Bazel test targets pass. The 107 unchanged targets retain their previously passing cached results, including native/WebAssembly conformance and historical differential coverage; both new targets execute successfully. Formatting and build-integrated lint checks pass. No production code changed, so this increment does not require a new runtime performance acceptance comparison.

```sh
bazel test -c opt //...
bazel run -c opt //:format -- --check
bazel run -c opt //benchmark:lifecycle -- expression '2*2*2*2*2*2' 2101 --sample 5
bazel run -c opt //benchmark:allocation -- expression '2*2*2*2*2*2' 2101 --sample 5
bazel run -c opt //benchmark:lifecycle -- exhaustive benchmark/case/token.wave --sample 5
```
