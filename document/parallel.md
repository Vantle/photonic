# CPU parallelism and the GPU decision

Measured on `eea981d`, after [waiting-span batching](batch.md), on the Apple M5 Max. This is a feasibility audit, not a new parallel runtime implementation. The existing exhaustive executor already distributes independent matching and normalization tasks and collects results in their original order. Direct path execution keeps synchronized firings under its existing coordinator.

## Existing executor

Three rounds alternate one, two and four workers. Every fixture warms for 100 ms, then records 25 native optimized samples. Measurements use `bazel run`, AC power, sleep inhibition and no competing builds. A changed sleep timestamp discards the execution. The [artifact](parallel.json) records each round's minimum, median and maximum; the existing runtime harness does not serialize individual timing samples. All nineteen fixtures agree on state, event, work, retained record and peak counts across worker counts. Kernel differential tests separately cover full snapshots and evidence.

| Exhaustive fixture | One worker | Two workers | Four workers |
| --- | ---: | ---: | ---: |
| Concrete Boolean deductions | 119.9 µs | 348.5 µs | 389.0 µs |
| Independent equal results | 139.8 µs | 346.4 µs | 401.3 µs |
| Nested body expressions | 59.8 µs | 137.1 µs | 156.8 µs |
| Escaped local definition | 347.6 µs | 592.3 µs | 631.5 µs |

Two workers are 1.70–3.45× slower across these small fixtures; four are 1.82–4.03× slower. Keep the existing default. These fixtures do not locate the crossover for large independent tasks, and do not establish that parallelism is always unprofitable. A future executor change needs a size/cost sweep with genuinely large independent work, cold controls and exact budgeted snapshots. Do not infer profitability from available core count alone.

```sh
bazel run -c opt //benchmark:runtime -- --workers 1
bazel run -c opt //benchmark:runtime -- --workers 2
bazel run -c opt //benchmark:runtime -- --workers 4
```

## Current arithmetic profile

A separate three-round, nine-sample diagnostic of six factors of two takes 67.77 ms with instrumentation. Ordinary execution in the batching audit is 63.3 ms. Median measured scope times are dispatch 38.12 ms, index maintenance 12.93 ms, rewrite 6.51 ms and matching 2.02 ms. Within dispatch, subscription is 25.21 ms, including preparation at 9.98 ms. Scopes nest; these values cannot be summed as independent phases. The raw diagnostic is included in the artifact.

Matching represents roughly 3% of instrumented execution. Even removing all of that measured phase would save only about 2 ms on this workload. This is an Amdahl-style estimate from the measured scopes, not a GPU measurement or a bound for larger joins. Subscription reconciliation and coherent index maintenance remain better targets for ordinary expression latency. Existing partial-order and causal-history proposals require much stronger semantic arguments before they can reduce required evidence or scheduling work.

```sh
bazel run -c opt //benchmark:profile -- '2*2*2*2*2*2' 2101 --sample 9
```

## Recommended next work

| Opportunity | Next concrete experiment | Acceptance boundary |
| --- | --- | --- |
| Subscription reconciliation | Attribute request construction, retained-accounting scans and consumer replacement separately; retain unchanged request components under exact dependencies | Real arithmetic improvement, late activation, lexical capture, removal/reinsertion and cold controls |
| Persistent fragment links | Compare immutable child references with copied transcripts on deep high-reuse joins | Bounded ownership and accounting, replay after eviction, searched-domain invalidation and exact scalar budgets |
| Exhaustive joined fragments | Share one exact multi-input fragment beneath consumer-specific proof projection | Preserve source, captures, executable reads, incompatible histories and private gates |
| Residual assignment filters | Reject impossible extensions under a partial binding without changing candidate visitation | Exact multiplicity, symmetry and suspension; skewed and productive controls |
| Contextual proof templates | Measure repeated nonidentity flow composition before designing a template cache | Fresh occurrence identities and complete competing provenance |
| CPU bulk work | Sweep independent task size and batch width through the existing executor | End-to-end crossover and ordered publication, not isolated thread throughput |

These are research and engineering candidates, not silently completed features. Structural creation of previously uncompiled rule shapes remains the separate language proposal. No GPU backend or new semantic feature is included in this delivery.

## GPU boundary

Proceed with a bounded feasibility experiment only after finding a large uniform workload that remains expensive after CPU reuse. The first candidate is read-only indexed filtering or intersection over an immutable snapshot, with the CPU retaining proof validation and synchronized commit. Compare complete latency including packing, submission, result transfer and integration. Apple's [command-buffer guidance](https://developer.apple.com/library/archive/documentation/3DDrawing/Conceptual/MTLBestPracticesGuide/CommandBuffers.html) explains why submission frequency and synchronization can dominate small workloads.

A portable browser experiment should use the current [WGSL specification](https://www.w3.org/TR/2026/CRD-WGSL-20260915/). Baseline atomic integers are 32-bit; a 64-bit `vec2<u32>` min/max facility is extension-dependent. Synchronization builtins have workgroup execution scope. Neither property supplies Photonic's global firing semantics. Preserve native identities through exact snapshot-local mappings or explicit pairs, and retain coordination outside individual shader workgroups.

The experimental boundary needs explicit snapshot generations, bounded output, exact CPU comparison, and fallback for overflow, stale snapshots, unsupported devices and cancellation. Measure sparse/dense/skewed input, repeated and changing snapshots, and native/browser integration. Enable acceleration only above a demonstrated end-to-end crossover. At present there is no measured GPU crossover and no evidence that offloading the current small arithmetic matching phase would produce a large gain.
