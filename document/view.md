# Incremental report materialization

Baseline: `b96868f`. Direct-path search, exhaustive runtime snapshots and prism reports now expose borrowed serializable views. The JSON CLI uses these views. This removes the need to retain a complete rendered report before writing it, while preserving owned snapshots for callers that need independent storage. Rules, programs and execution scheduling are unchanged.

## Representation and contract

`path::Search::view`, `runtime::Runtime::view` and `prism::Search::view` return serializable values borrowing a consistent engine state. Rust's borrow prevents execution from mutating that engine while a view remains in use. Creating a view does not render the history. Serialization visits the state, event and projection sequences incrementally and releases each temporary rendered item after serialization.

The existing `report` and `snapshot` methods still return fully owned values that can outlive the engine. Report containers now have generic payload parameters with owned defaults, allowing both forms to use the same field definitions and derived serialization. Callers explicitly constructing these containers can annotate the owned type when collection inference needs it. The normal owned return types retain their existing fields and ownership. JSON field order, omissions and values are unchanged.

The private [sequence adapter](../language/render/sequence.rs) creates a fresh iterator for every serialization. A view is repeatable rather than a consumable cursor. The [direct-path renderer](../language/path/report.rs) and [runtime renderer](../language/runtime/report.rs) share their item construction between owned collection and incremental serialization. Prism composes the runtime view with borrowed program and target data. No second evaluator or duplicated JSON schema is introduced.

Direct-path serialization retains the existing canonicalization order, frame-sharing table and canonical mappings. It warms the same canonical and event caches as complete owned reporting. The table and presentation-text dictionary belong to the iterator and are released when traversal ends. The engine still retains history and canonical forms; this is not constant-memory execution or canonical-history eviction.

Writer errors propagate through the serializer. A failed export can leave a valid prefix of the ordinary inspection caches initialized. Retrying starts a fresh iterator, and a complete retry produces the same output as owned reporting. The new view operation does not promise transactional rollback of inspection caches on writer failure. Successful full JSON export followed by resumed execution preserves the tested work, statistics, state and provenance observations.

The CLI retains owned reports for its text presentation. Its JSON paths use the borrowed views for direct-path, exhaustive and prism output. Native and WebAssembly builds use the same implementation, with explicit Bazel source lists and no new dependency or unsafe code.

## Measurement

The [raw audit](view.json) contains baseline and candidate executable hashes, source hashes, all timing and allocation samples, exact report comparisons, fixture contents and reproduction scripts. Measurements run serially on the current Apple M5 Max on AC power after builds and tests finish.

The lifecycle benchmark now accepts `--export owned` or `--export view`, with optional `--writer`. Both modes use the same serializer and output adapter. Buffered mode retains the encoded byte vector; writer mode discards output after updating its byte count and checksum. This measures serialization and rendering rather than operating-system or terminal throughput. Checksumming adds time, so these durations should not be compared directly with the older lifecycle benchmark's durations.

Each complete measurement includes initialization, execution, export and release. Owned export includes rendering, serialization and destruction of the temporary owned report. Three rounds alternate mode order, with a cold execution recorded separately and five fresh-instance samples per process. Allocation instrumentation runs separately with three samples; timings from those runs do not determine acceptance.

| Six-factor complete export | Owned report | Borrowed view |
| --- | ---: | ---: |
| Buffered peak requested bytes | 372.24 MB | 264.45 MB |
| Writer peak requested bytes | 238.03 MB | 130.23 MB |
| Buffered lifecycle | 759.43 ms | 763.69 ms |
| Writer lifecycle | 696.95 ms | 689.69 ms |

Buffered peak falls 29.0%, and writer peak falls 45.3%, avoiding about 108 MB of temporary rendered history. Latency is approximately flat: buffered export is 0.6% slower and writer export 1.0% faster, within the existing tolerance. The 64-level scope fixture reduces writer peak from 3.36 MB to 0.84 MB, about 75%. Small fixtures whose execution already dominates peak may show little or no peak reduction.

All 22 export comparisons pass the existing 5% latency tolerance, or 10% below one millisecond. They cover eleven workloads in both buffered and writer modes, including arithmetic, nested scopes, long chains, exhaustive proof cases and uniform/distinct large states. Encoded length and checksum agree across modes, allocation instrumentation and cold/warm samples. Every allocation diagnostic returns to zero additional tracked retained bytes after release. Requested bytes exclude allocator metadata, stack storage and process RSS.

The existing 42 protected comparisons also pass after a calibrated follow-up for one very small lifecycle control. Initial processes with five samples each yielded pooled medians of 21.12 versus 24.13 microseconds, exceeding the submillisecond gate. Three alternating processes per side with 501 samples each give 14.25 versus 14.08 microseconds and preserve every observation. Both datasets are retained; the follow-up changes the warm-sample evidence, not the tolerance. The other 41 comparisons pass their original matrix. Compact six-factor execution is approximately unchanged at 59.94 versus 60.01 ms, as expected for a reporting change.

## Verification

All 109 optimized Bazel test targets pass, with 103 freshly executed and six cached, including native/WebAssembly conformance. The kernel contains 222 passing tests. New report tests compare compact and pretty JSON repeatedly, exercise empty and cyclic cases, nested captures and competing histories, interleave reporting with bounded execution and tight limits, preserve owned snapshots after engine destruction, and verify writer-error propagation and retry followed by resumed execution.

Twenty-two complete and paused command reports compare byte-for-byte with the baseline, including the six additional exhaustive prism comparisons. No fields are filtered. The existing 128-step dispatch verification trace also agrees. New benchmark tests compare owned/view and buffered/writer output across ordinary and allocation-instrumented binaries, reconcile every allocation phase with the reported footprint, and verify complete release. Formatting and build-integrated checks pass.

```sh
bazel test -c opt //...
bazel run -c opt //:format -- --check
bazel run -c opt //benchmark:lifecycle -- expression '2*2*2*2*2*2' 2101 --export owned --sample 5
bazel run -c opt //benchmark:lifecycle -- expression '2*2*2*2*2*2' 2101 --export view --sample 5
bazel run -c opt //benchmark:allocation -- expression '2*2*2*2*2*2' 2101 --export view --writer --sample 3
```

The report-view architectural milestone is delivered across the three JSON report consumers. The first implementation still constructs one rendered item at a time, and the engine retains canonical history. Any further representation change should target those measured costs separately. Subscription lifecycle maintenance remains the leading execution opportunity; this change does not establish an execution-speed improvement.
