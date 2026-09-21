# CPU continuation

This work extends `3bec4b2d72b8c2a2981bf9602e620f83a681a0ac`. It does not introduce GPU execution, arithmetic shortcuts, syntax changes, or a language execution order.

## Acceptance

Every accepted implementation must preserve exact bindings, captures, source evidence, synchronization, logical work and suspension boundaries. Cache storage may differ; cache pressure must retain an exact fallback. Native Bazel measurements compare sequential alternating runs against the checkpoint. Protected existing workloads use a 5% regression threshold, or 10% below one millisecond, following the checkpoint audit. New microbenchmarks expose construction as well as reuse costs. Focused gains are not arithmetic speedups.

## Sequence

1. Refresh expression attribution and investigate subscription/index allocation overhead.
2. Share immutable trace records with independent mutable continuation state.
3. Reuse completed exact exhaustive matching transcripts across compatible snapshot projections, preserving every pending step and private proof projection. This is a bounded matching-fragment implementation, not arbitrary incremental multi-query joins.
4. Compile reusable contextual flow-construction templates with explicit parent provenance and fresh destination mapping.
5. Measure and admit expensive independent search steps through the existing ordered CPU executor.
6. Investigate stronger residual assignment certificates and causal-history compression with explicit counterexamples and preservation requirements.

## Initial findings

Nine instrumented samples of six factors of two give a 70.08 ms median. Median scopes are dispatch 39.43 ms, index 13.23 ms, rewrite 6.84 ms and matching 2.06 ms. Subscription is 26.32 ms inside dispatch, including 10.07 ms of preparation. Scopes overlap and must not be summed. These timings include instrumentation and are diagnostic attribution, not the powered acceptance run.

Small reconciliation buffers and delta-only consumer accounting did not improve end-to-end expression latency. A reusable request buffer also remained flat, and join controls regressed beyond tolerance. These experiments were removed. They do not count as shipped improvements.

A direct persistent-vector trace experiment substantially reduced snapshot costs but increased construction and traversal costs. The accepted representation stores empty and single-record traces inline, keeps other short traces flat, and separates longer traces into immutable 32-record chunks and a private mutable tail. Full benchmark results follow below.

## Implementation

| Layer | Ownership and invariant |
| --- | --- |
| `joining/transcript.rs` | Inline single records, flat short traces, persistent directory of sealed chunks, private tail; cloning never shares mutable continuation |
| `joining/trace.rs` | Existing budget reservations and exact waiting length; cached binding count removes repeated accounting scans |
| `search/cursor.rs` | The original exhaustive enumerator, including its gate, particle cursor and pending steps |
| `search/key.rs` | Exact pattern and capture, input order, candidate world position and weak immutable occurrence identity |
| `search/transcript.rs` | Completed bindings and run-length-encoded waiting records; each consumer advances independently |
| `search/replay.rs` | Recording, playback and exact fallback; an unfinished recording never becomes a cached negative answer |
| `search/store.rs` | Bounded publication, dead-key pruning and reservations retained until the final reader releases an entry |
| `flow/template.rs` | Groups equal source sets and destination mappings; evaluates each union once against the current parent |
| `flow/store.rs` | Separate weak parent and event identities, bounded template/union storage and explicit eviction |
| `work.rs` | Admission for expensive independent normalization or unshared search preparation; ordered result publication remains in the runtime |

### Trace storage

Snapshots share sealed record chunks through a persistent directory and copy at most 32 tail records. Existing binding payload sharing remains in place. Empty and single-record traces need no record-vector allocation; other short traces retain contiguous storage. Header and binding counts use bounded 16-bit fields because the existing 4,096-unit trace limit bounds both values. This narrows representation without adding a language limit. Parent extension still constructs destination record headers and prefix links; it is not a lazy persistent representation of every composition. Existing logical record reservations remain conservative even when physical storage is shared.

The direct join also caches immutable negative particle preparations and maintains an occupied-site mask below site 64, with an exact scan above that boundary. Neither optimization skips a logical step. The [certificate investigation](certificate.md) explains the negative transition. A compiler regression required making the outer `Join::step` boundary explicitly inlineable: assembly inspection showed that the candidate had introduced an out-of-line call on every cheap pending step. Unhelpful cursor and playback outlining experiments were removed. Sequential chunk-iterator experiments also regressed short-trace controls and were removed; the retained iterator uses exact indexed traversal.

### Exhaustive reuse

Admission requires multiple inputs and at least 32 pattern terms. Replay state is allocated only for admitted searches, keeping ordinary cursor values compact. Keys larger than 4,096 logical units or transcripts exceeding their bounded payload allowance use the private enumerator. Stores hold at most their configured reservation capacity. Publication happens only after exhaustion. Weak occurrence keys prevent address reuse from equating distinct resources without retaining dead world payloads.

Reuse is valid across snapshots only when the complete candidate projection and traversal order agree. Changes to irrelevant worlds are permitted. Changes to relevant identity, capture, pattern, token identity or position either change the key or are reflected by a new immutable occurrence. Replayed slots still feed the consumer's own proof projection.

Evicting a live replay restores the private cursor by advancing it to the exact recorded progress, then releases replay ownership. This reconstruction is extra implementation work, not extra logical runtime work. It can be expensive for a long transcript; preserving a full private gate snapshot would trade that cost for more retained memory. Cache saturation and incomplete producers always fall back to enumeration.

This implements completed whole-query reuse, not arbitrary interior subqueries or incrementally maintained row relations. Those remain separate architectural work. Key construction still scans the candidate projection, and playback still emits each pending step separately. Reuse reduces the work inside those steps; it does not remove the logical enumeration lower bound. Scheduler-aware consumption of cached waiting spans is a possible next experiment, but must preserve queue rotation, publication order and partial-budget continuation across all active consumers.

### Contextual construction

Templates group exact resource and context source sets, retain the event's destination/frame mapping, and instantiate against the current immutable parent. They do not cache fresh rewrite resources or merge proof-source identities. Compilation is deferred until the same immutable event is observed a second time, avoiding template construction for one-shot calls. The existing large-flow admission gate remains: at least 32 output resources and at least one source set with eight members. Expired weak keys are pruned; record-pressure eviction disables retention with exact direct composition available.

### CPU execution

The existing executor now considers an unshared search preparation substantial when its world has at least 4,096 tokens. The estimate multiplies token count by pattern width capped at eight. Warm shared preparation and transcript playback are excluded. Batch admission still requires two substantial tasks and at least 8,192 estimated visits. Tasks advance once and their results are published in the original order. This does not impose an execution order on the language or change synchronization.

The benchmark measures 32 independent search steps, with one- and eight-term patterns, through three successive advances. Construction and result validation are outside the timed intervals. Worker-count gains describe these batches, not an end-to-end program speedup. The default runtime remains single-worker.

## Larger changes

The [residual and causal investigation](certificate.md) records concrete counterexamples, dependency requirements and experimental acceptance gates. General residual pruning cannot discard pending work merely because no answer exists below a branch. Causal reduction cannot discard inspectable histories merely because terminal values agree. The next defensible experiments are exact progress certificates for restricted residual suffixes, and compressed history representations with lazy reconstruction of all required observations. Neither general optimization is enabled here.

Subscription/index changes remain a measured opportunity, but this pass found no acceptable end-to-end gain. Cross-plan interior query reuse and fully lazy trace composition also remain open. GPU execution is outside this work.

## Research basis

[Shared Arrangements](https://www.vldb.org/pvldb/vol13/p1793-mcsherry.pdf) motivates separating retained reusable data from consumer progress. [Free Join](https://arxiv.org/abs/2301.10841) supports measuring an adaptive mix of join strategies rather than assuming one universal replacement. [Unfolding-based Partial Order Reduction](https://arxiv.org/abs/1507.00980) provides a starting point for the later causal investigation. These papers do not establish equivalence for Photonic's resource, evidence and bounded-progress contracts.


## Measured acceptance

The [raw audit](continuation.json) contains the three-round full comparison, source hashes, separate final layout refinements, longer-duration controls, focused measurements and rejected experiments. The full matrix measures implementation commit `3045cb1`; the subsequent compact replay representation is checked across ten targeted workloads, and the final inline step wrapper across four. The final focused measurements have their own source manifest. Every shipped source change is covered by the fresh full test suite.

| Workload | Checkpoint | Candidate | Result |
| --- | ---: | ---: | --- |
| Six factors of two | 59.62 ms | 59.14 ms | Approximately unchanged |
| Ten-digit decimal addition | 187.69 ms | 187.72 ms | Approximately unchanged |
| Five-digit decimal multiplication | 207.45 ms | 206.72 ms | Approximately unchanged |
| Single-record snapshots | — | — | 2.1–2.2× faster |
| Single-record parent construction | — | — | 1.2–1.4× faster |
| 256-record snapshots | 14.64 ms | 2.55 ms | 5.7× faster |
| Repeated flow construction | — | — | 10.8–18.1× faster than checkpoint reuse |
| Repeated exhaustive queries, 32–512 inputs | — | — | 2.5–4.5× faster than private queries |
| Large search batches, four workers | — | — | 1.7–4.0× faster than one worker |

Trace timings cover 32,768 operations; flow timings cover 128 compositions; exhaustive reuse covers 64 complete queries. CPU batch measurements cover 32 searches through three successive steps. Their ratios have different denominators and must not be multiplied or described as arithmetic speedups. Measurements come from one Apple Silicon laptop, not all supported platforms.

The final protected results, combining the full matrix with the explicitly identified later refinements, meet the 5% threshold, or 10% for submillisecond controls. Earlier regressions and individual-round outliers remain in the raw data. A longer-duration follow-up runs preparation and exhaustive controls with 16,384 iterations: approximately 4.4% and 2.4% overhead before the final wrapper refinement. The final ordinary exhaustive fixture measures approximately 1.6% overhead. These repetitions help distinguish startup variability from sustained cost; they do not replace the original samples.

Two new stress cases expose remaining costs. Constructing a parent from 256 records takes 138.51 ms versus 119.74 ms, about 16% slower, while snapshotting that trace is 5.7× faster. Persistent snapshots do not yet make parent composition lazy. Cold 512-input exhaustive recording takes 35.45 ms versus 32.72 ms, about 8% overhead; repeated use takes 7.32 ms. Those costs are explicitly accepted tradeoffs in the new workload coverage, not removed observations or claimed universal wins. More selective cold-query admission and lazy parent composition remain opportunities.

The first subscription/index allocation experiments produced no acceptable end-to-end gain and were removed. Eager flow-template compilation regressed cold controls; deferring compilation until reuse restored those controls. Iterator variants regressed short traces; inline single-record storage resolved the snapshot cost while preserving a simpler indexed iterator. Optional replay storage is boxed only for admitted consumers, and the small search-step wrapper is inlineable.

## Verification and reproduction

The final source passes all 107 optimized Bazel test targets, 207 debug kernel tests and 875 historical budgeted observations. This includes native/WebAssembly conformance, metaprogramming, synchronization, support and cache-pressure coverage. New tests check persistent snapshot lifetime and tail isolation, replay at every eviction boundary, capture and occurrence identity, saturated stores, site identifiers above the mask width, and serial/parallel search-step equivalence.

Historical observations exclude only exhaustive cache `record` and `peak` fields. Bindings, work, suspended queues, states, captures, flows, evidence and support remain compared. Passing these checks establishes no known semantic regression in the tested domain; it does not prove every program or the reference kernel correct.

```sh
bazel test -c opt --nocache_test_results //...
bazel test --nocache_test_results //language:test
bazel run -c opt //benchmark:differential
bazel run -c opt //benchmark:expression -- '2*2*2*2*2*2' 2101 --sample 9
bazel run -c opt //benchmark:segment
bazel run -c opt //benchmark:transcript
bazel run -c opt //benchmark:composition
bazel run -c opt //benchmark:search
```

The integrated comparison uses separate checkouts and sequential native `bazel run -c opt` calls in three alternating rounds. The checkpoint receives only the same additional long-record segment fixtures. AC power and unchanged sleep counters are required before and after each measurement, with idle and system-sleep prevention active on AC for the final sweep. Earlier timings affected by the reported power interruption are excluded. Subsequent AC and sleep interruptions also stopped the harness; their partial rounds were discarded before restarting. Test results were freshly rerun after the original interruption.

A final debug run was interrupted after all 207 test bodies passed, causing a Bazel wall-clock timeout. It was rerun under sleep prevention and passed cleanly in 8.4 seconds. Linux Buildkite build and test verification passed for `3045cb1`; the protected PR requires verification again for the submitted final head.
