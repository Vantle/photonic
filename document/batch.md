# Exact batching of cached waiting spans

This stage follows [exhaustive preparation sharing](exhaustive.md), with baseline `d9020d8`. Cached matching transcripts already compress consecutive pending polls in storage. The executor can now consume those same spans in one operation while preserving every logical budget boundary. Scalar matching remains available as a reference.

## Preservation boundary

A batch consists exclusively of already recorded pending polls. It cannot cross a binding, completion, recording frontier, new cache publication or synchronized firing. Preparing a new fragment and recording a new result remain scalar. Retained records do not change inside an eligible span.

`joining/playback.rs` owns the cursor arithmetic. Single-depth partitions, multilevel replay, whole-prefix replay and their product adapters expose only the pending span currently available without changing another consumer. `replay.rs` preserves the same boundary for shared complete plans. `dispatch/entry.rs` rejects batching while a binding awaits delivery to any consumer.

`dispatch::Network` preserves its round-robin agenda. When every entry has a pending span, it can consume whole agenda cycles up to the shortest span and caller budget. Otherwise it consumes one contiguous pending prefix and rotates exactly that prefix to the back. It requires at least two logical steps; after an unsuccessful probe, it waits 31 scalar opportunities before probing again. This bounded cooldown only controls discovery overhead, never logical progress or feature availability.

`reduction::Search` permits batching only after initialization and when no event awaits delivery. `path::Search` checks termination, limits and initial canonicalization first, and batches only when no event or candidate is pending. Both counters advance by the exact number of skipped polls. Because retained storage cannot change inside the span, the preceding record-limit check is equivalent to the repeated scalar checks. A budget ending anywhere in the span leaves the same cursor and observable state as scalar execution.

This preserves unordered Photonic state and synchronized gate passing. The existing implementation's deterministic observation order remains intact; no language-level execution order is introduced.

## Validation

Playback tests enumerate offsets and bounded skips across waiting and binding records. Fragment differential tests now consume bounded batches against a direct scalar oracle, including context changes, capture-bearing prefixes, replacement, negative fragments, cache pressure and eviction. Dispatch tests compare exact deliveries, agenda rotation and retained accounting with merged consumers and changing inputs. Path tests compare complete reports and statistics at arbitrary budget boundaries, including zero/one-step budgets and forced record-limit eviction.

All 106 optimized Bazel test targets passed, including native/WebAssembly conformance. All 192 debug kernel tests passed, as did formatting and diff checks. The protected benchmark matrix preserves maintenance work, binding and peak counts, complete subscription observations, and state/event/work/record/peak across nineteen exhaustive fixtures. Tests provide evidence of preservation, not a proof that no kernel bug can exist.

## Measurements

The [raw audit](batch.json) contains three alternating paired rounds, nine samples per parameterized case, source hashes and the baseline adapter. All runs use optimized native Bazel execution on the Apple M5 Max, AC power and sleep inhibition. A changed `kern.sleeptime` invalidates a sample run; the accepted audit contains no interruption. Measurements below are pooled execution medians.

| Workload | Before | After | Speedup |
| --- | ---: | ---: | ---: |
| Interior partition | 91.4 ms | 43.7 ms | 2.09× |
| Deeper partition | 151.7 ms | 47.3 ms | 3.21× |
| Alternating interior updates | 78.1 ms | 27.7 ms | 2.82× |
| Projected negative context | 31.1 ms | 12.2 ms | 2.54× |
| Productive context | 33.1 ms | 30.5 ms | 1.09× |
| Sustained eight-level reuse, 1,024 updates | 23.0 ms | 5.7 ms | 4.07× |
| Sustained twelve-level reuse, 1,024 updates | 308.7 ms | 28.6 ms | 10.77× |
| Six factors of two | 64.3 ms | 63.3 ms | 1.02× |
| Ten-digit decimal addition | 195.1 ms | 193.6 ms | 1.01× |
| Five-digit decimal multiplication | 217.1 ms | 215.3 ms | 1.01× |

All protected pooled medians passed the declared five-percent regression gate, with ten percent allowed for submillisecond controls. Replacement was 3.6% slower. Scalar controls were 1.4–2.5% slower. An earlier version probed unproductive agendas too frequently; the bounded cooldown removed the larger prefix/churn overhead. These are workload-specific gains, not multipliers to apply to the earlier stages or arbitrary programs.

The partition/context benchmarks isolate repeated join maintenance. Their default uses the batch primitive; `--scalar` retains per-poll traversal in the same candidate binary. End-to-end arithmetic and prefix cases exercise runtime integration independently.

```sh
bazel run -c opt //benchmark:expression -- '2*2*2*2*2*2' 2101 --sample 9
bazel test -c opt //...
bazel test //language:test
```

These benchmarks were removed after `0daf296` and still run at that commit:

- [`partition`](https://github.com/Vantle/photonic/blob/0daf296e1d126675ed445bfb7ed6674a379b1532/benchmark/partition.rs) with `--depth 12 --width 2 --count 2 --alternating --length 1024 --sample 9` and `--depth 12 --width 2 --count 2 --alternating --length 1024 --scalar --sample 9`
- [`context`](https://github.com/Vantle/photonic/blob/0daf296e1d126675ed445bfb7ed6674a379b1532/benchmark/context.rs) with `--sample 9`

## Remaining boundary

Batching applies to cached waiting spans in direct path evaluation. Exhaustive proof tasks retain their existing scalar progress and ordered publication. New matching work, new bindings, synchronization and proof construction are not skipped. Persistent child-link fragments, broader cross-plan joins, residual assignment filtering and contextual proof templates remain conditional algorithmic work. The next evaluation is the existing CPU executor's crossover, followed by a measured GPU feasibility decision.
