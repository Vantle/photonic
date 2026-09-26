# Interior matching and unfinished sharing

This delivery follows the [root-partition audit](partition.md), using baseline `d7fd98aacadc92fffe148d407cd91df90c921b32`. The [measurement artifact](interior.json) records source manifests, native commands, raw samples, and machine configuration. The [CPU research review](research.md) evaluates the remaining architectural opportunities before GPU execution.

## Implemented boundary

Completed join fragments can now survive changes below the first input. A fragment is keyed by the candidate at a chosen traversal depth and the exact binding already selected before it. An occurrence-to-fragment reverse index retracts only the records that mention a removed site. Surviving records remain available when their other searched domains and traversal order are unchanged.

The partition depth adapts to the deepest changed prefix input. Moving the partition deeper discards its older records, because those records depended on the newly changed searched domain. This is one adaptive partition depth per join, not an arbitrary multilevel materialized join graph. It does not yet share individual partition records across plans or integrate them into exhaustive proof matching.

Admission retains the existing repeated-update and wide-pattern gates. It also requires an unchanged traversal order, a surviving occurrence in each changed prefix domain, and multiple candidates at the chosen depth. The partition must contain another prefix input below its anchor or a wide particle at the anchor. A final narrow prefix particle has too little remaining work to justify joined-fragment bookkeeping and uses direct traversal. This policy changes acceleration only; all bindings remain enumerable through the fallback.

Whole-prefix consumers can now share unfinished enumeration through immutable snapshots. The producer continues to own its mutable cursor and private recording. Snapshots are published at geometric progress checkpoints, beginning at 256 steps. Each follower owns its playback position. At a snapshot frontier it takes a longer published snapshot if available; otherwise it reconstructs its private cursor to the delivered position and continues exact traversal. No consumer speculatively drives another producer, and an unfinished snapshot never establishes absence.

General join traversal now checks symmetry against the last selected equivalent input. All earlier selections in that equivalence class already form a strictly increasing occurrence sequence, so transitivity makes the earlier comparisons redundant. Distinct-world checks across all inputs remain intact. This removes repeated rank comparisons without changing candidate order or the progress stream.

## Ownership and interfaces

| Component | Owns |
| --- | --- |
| `joining::Join` | Query lifecycle, traversal choice, current ordinal projection |
| `joining::strategy` | Admission from a named request containing domains, order, changes, index and existing depth |
| `joining::Cursor` | Exact enumeration, ancestor binding and valid subtree boundaries |
| `joining::Dependency` | Anchor occurrence and exact selected-prefix identity |
| `joining::Retention` | Completed fragment records and occurrence-to-fragment reverse membership |
| `joining::Partition` | Active fragment, replay, retraction, bounded recording and restoration |
| `joining::Recording` | Private recording, immutable publication, aggregate allowance and publication cadence |
| `joining::Stream` | Producer/follower progression and fallback to a private cursor |
| `joining::Trace` | Compressed pending runs, exact binding selections, logical length and budget ownership |
| `joining::Playback` | Independent delivery and seeking within an immutable transcript |
| `joining::Node` | Revision-scoped weak publication; an older or shorter transcript cannot replace a live longer one |
| `Product<Prefix>` | Composition of a prefix strategy with exact suffix enumeration |

Mutable cursor state remains private. The producer constructor accepts an owned `Trace`; the follower constructor accepts an immutable shared trace. Snapshot allocation is fallible and reserves its complete logical footprint before copying. Reverse-index ownership is separate from traversal policy. Bazel lists every new source explicitly; no dependency or frontend build behavior changed.

## Preservation argument

1. At a partition boundary the cursor has exactly the selected ancestor binding and no active candidate scan at the partition depth. A fragment enumerates one candidate's entire subtree until the next such boundary. Seeking past a completed fragment retains ancestor cursor state and clears only the deeper traversal positions.
2. The key includes stable sites and exact token identities for the selected ancestors, plus the anchor site. The owning join supplies immutable patterns, captures and unchanged searched domains below the partition. Relative occurrence order among survivors is unchanged.
3. Every removed site is retracted before enumeration resumes, including numeric slots reused by an insertion in the same update. Removal follows reverse dependencies for both anchors and selected ancestors. An equal replacement value does not revive a retired occurrence's record.
4. Deeper searched-domain changes invalidate the older partition. This covers negative fragments as well as successful bindings. Candidate-domain overlap is included when choosing the deepest changed input; a world cannot secretly alter an untracked descendant domain.
5. Replay delivers the same ordered pending/binding sequence. It preserves token identity and resolves current world ordinals at the existing query boundary. The suffix still checks distinctness and symmetry using the current index. Executable reads and consumer-specific proof projection remain owned by their existing outer layers.
6. Snapshot publication never exposes a trace that the producer will subsequently mutate. A complete trace is immutable; an incomplete publication is a separately reserved copy. Within a revision the node retains the longest live publication. Revision lookup prevents unrelated snapshots from being reused; unchanged-prefix promotion remains governed by join invalidation.
7. A follower delivers every record independently. Restoration replays the already delivered logical prefix internally before requesting the next result, so it neither omits nor duplicates externally delivered steps. Reset and eviction preserve the existing continuation contract.
8. Cache exhaustion, failed publication, and ineligible admission fall back to exact traversal. Neither storage pressure nor the recording-length ceiling truncates answers. The symmetry simplification changes only the rejection predicate's cost, not its result.

State remains unordered. Traversal order is an implementation detail, and synchronization still establishes a firing. No arithmetic program is recognized specially, no frontend expectation changed, and no production resource limit increased. Existing dynamic executable-rule and deep metaprogramming behavior remains covered by the suite; future structural code-construction syntax remains the separate [dynamic proposal](dynamic.md).

## Accounting and performance boundaries

A prefix retains at most 4,096 logical records, including fragment dependency keys and reverse memberships, or the producer's private trace plus its owned publication. Every trace and snapshot also reserves from the existing shared 65,536-record budget. Each recording spans at most 65,536 logical progress steps. Dependency metadata and reservations are released on retraction, reset of an incomplete partition, saturation, eviction or destruction as appropriate.

Logical records are not allocator bytes. A follower can retain an older immutable snapshot after the producer releases it; shared budget ownership remains with that trace until its last owner disappears. Reported per-query retention can include shared storage, as in the preceding implementation. The audit does not measure allocator peak bytes.

The work counter is intentionally unchanged. Replay saves matching computations while continuing to deliver each required logical progress boundary. A follower that outruns a paused producer must reconstruct a private cursor; in that case snapshot delivery can cost more than unshared traversal. This is a bounded reuse tradeoff, not shared cooperative frontier production.

## Validation

All 106 optimized Bazel test targets pass, including native/WASM browser conformance and the memory-heavy repeated ternary test. The 172-test kernel also passes without optimization. Configured Rust formatting, Clippy and Buildifier checks remain enabled.

Six new kernel tests add interior retraction with changed token identities; eviction at 192 offsets across three partition depths with ordinary and captured rule values; repeated partial enumeration, depth changes and complete replacements under several budgets; unfinished producer/follower interleavings; immutable snapshot seeking; and mutation across revisions under memory pressure and opposite update orders. Existing tests cover saturation, direct/exhaustive comparison, nested rule values and occurrence reuse. Every compared join result includes exact bindings and progress, with independent retention checks. The previous all-predecessor symmetry predicate is evaluated as an equivalence assertion throughout kernel tests, independently of the optimized predicate.

The final local commands are:

```sh
bazel test -c opt --nocache_test_results //...
bazel test --nocache_test_results //language:test
```

## Native measurements

Measurements use sequential alternating optimized native `bazel run` pairs on an Apple M5 Max, with idle sleep inhibited and all binaries built before timing. The final audit and confirmation ran on AC power, checked before and after every invocation. Earlier battery measurements are excluded from this report. Three rounds provide 27 samples per side for scalar workloads. The identical benchmark harness is copied to an isolated baseline checkout; its production runtime is unchanged. Timing below concerns execution, not Bazel startup or source parsing.

The partition harness varies the mutable input's depth, domain size, replacement fraction and productive/negative result. Its timer includes coherent index updates, join updates and complete enumeration. The frontier harness measures follower delivery after producer warmup and a bounded partial advance; it excludes producer construction and publication time, so its speedup is not an end-to-end expression speedup. Unshared and outrunning controls accompany it. Protected workloads include prefix churn, complete sharing, arithmetic, candidate preparation, leaf reuse, candidate maintenance and all 19 exhaustive fixtures.

| Workload | Before, ms | After, ms | Ratio |
| --- | ---: | ---: | ---: |
| Root update, width 12 | 69.9523 | 65.0469 | 1.075× |
| Interior update, width 4 | 0.4978 | 0.4153 | 1.199× |
| Interior update, width 8 | 12.6791 | 6.4224 | 1.974× |
| Interior update, width 12 | 277.5905 | 130.2662 | 2.131× |
| Depth-two update, width 12 | 577.9299 | 256.3792 | 2.254× |
| Interior update, eight candidates | 563.0770 | 194.6340 | 2.893× |
| Productive interior update | 279.8735 | 132.9351 | 2.105× |
| Complete replacement control | 276.9248 | 194.7532 | 1.422× |
| Partial delivery, unshared | 2.6676 | 1.9295 | 1.383× |
| Partial delivery, shared | 2.6726 | 0.4849 | 5.512× |
| Outrunning control, unshared | 10.0051 | 7.2987 | 1.371× |
| Outrunning producer, shared | 10.0092 | 7.5325 | 1.329× |
| Shared candidate filters | 10.2204 | 10.2959 | 0.993× |
| Progressive activation | 14.3706 | 14.4844 | 0.992× |
| Stable prefix control | 4.5442 | 3.9535 | 1.149× |
| Changing prefix control | 5.0163 | 4.3465 | 1.154× |
| Six factors of two | 69.9018 | 68.4903 | 1.021× |
| Decimal 1234567890 + 9876543210 | 206.4613 | 204.5367 | 1.009× |
| Decimal 12345 × 67890 | 231.7393 | 228.4615 | 1.014× |
| Leaf reuse | 3.5048 | 3.5079 | 0.999× |
| 4,096-candidate domain | 12.3012 | 12.2382 | 1.005× |
| Stable prefix vs. pre-partition revision | 4.2991 | 3.9471 | 1.089× |
| Changing prefix vs. pre-partition revision | 4.7611 | 4.3608 | 1.092× |

Ratios above one are faster. Scalar rows pool 27 samples per side. The last two rows use seven additional three-revision rounds, totaling 357 samples per revision and workload, against pre-partition revision `3961d12f80ac7daf72943fdc170d4cfcad763f8b`. They confirm that the inherited regression is removed, rather than merely comparing favorably with its slower immediate parent.

Interior maintenance is 1.20–2.89× faster across the measured sizes, including 2.11× when actual bindings are produced. The general symmetry improvement also accelerates full replacement and unshared traversal. Shared partial delivery is 5.51× faster than baseline; within the new revision it is 3.98× faster than the unshared control. When followers overrun the producer, shared delivery is about 3.2% slower than the same-revision unshared control because reconstruction repeats saved traversal. Its improvement over the old runtime comes from the general cursor optimization, not a successful shared-frontier algorithm.

The declared investigation threshold is a repeatable slowdown above 5%, or above 10% for submillisecond samples. Protected pooled medians remain within those tolerances: candidate filtering, activation, leaf reuse and domain maintenance are within 1%, and arithmetic is within about 2%. The smallest join control rises from 35.6 to 38.2 microseconds, a 7.4% slowdown within the submillisecond tolerance. Existing isolated joining cases range from 0.931× to 1.357×; completed sharing cases range from 0.980× to 1.329×. These are scoped improvements, not a claim that every program became faster.

All paired logical work/binding and arithmetic event/work counts agree. The width-12 interior case delivers 29,353,920 logical steps, retaining 790 records at update boundaries versus 716 before. The productive case delivers 6,144 bindings and 29,452,224 steps; its observed retained peak rises from 694 to 4,416 across the complete join, including its acceleration structures. Partial shared delivery delivers the same 253,952 steps and retains 8,166 records across the measured consumers versus 16,565 before. These are observation-boundary retention values, not allocator peaks or every-step memory profiles.

All 19 exhaustive fixtures preserve identical state, event, work, record and peak counts. Median per-case timing ratios are 1.004× for ordinary execution and 1.008× for instrumented proof execution. Their respective ranges are 0.989–1.031× and 0.983–1.028×. This direct matching work does not claim a major exhaustive-proof speedup.

The initial broader admission policy exposed 1.5–6× slowdowns when it tried to retain a final narrow prefix particle. That experiment was rejected. The final policy requires meaningful remaining prefix work, and the audited changing-join controls now improve. Boxing the direct cursor and outlining partition dispatch also failed to resolve the protected prefix regression and were reverted before the final audit.


Reproduce focused cases with:

```sh
bazel run -c opt //benchmark:prefix -- --sample 31
bazel run -c opt //benchmark:prefix -- --changing --sample 31
```

These benchmarks were removed after `0daf296` and still run at that commit:

- [`partition`](https://github.com/Vantle/photonic/blob/0daf296e1d126675ed445bfb7ed6674a379b1532/benchmark/partition.rs) with `--depth 1 --width 12 --sample 9`, `--depth 2 --width 12 --sample 9`, `--depth 1 --width 12 --productive --sample 9` and `--depth 1 --width 12 --replacement 4 --sample 9`
- [`frontier`](https://github.com/Vantle/photonic/blob/0daf296e1d126675ed445bfb7ed6674a379b1532/benchmark/frontier.rs) with `--sample 9` and `--length 32768 --sample 9`

## Follow-up diagnostic

The instrumented native expression profile attributes about 38 ms to dispatch, 13 ms to index maintenance and 2 ms to the matching phase within an approximately 70 ms execution. Subscription and preparation are nested inside dispatch, accounting for about 26 ms and 11 ms respectively. Seven raw samples are retained in `interior.json`; instrumentation and nesting make these diagnostic values distinct from the paired uninstrumented timings. This supports targeting subscription/preparation reuse before matching-only GPU acceleration. The research review explains the qualification and next steps.

## Remaining boundary

The requested delivery implements interior dependency ownership, safe immutable sharing of unfinished transcripts, and the protected prefix improvement. It does not complete every research track. Shared immutable particle preparation, multilevel maintained fragments, checkpointed continuation sharing, exhaustive-fragment projection, contextual construction, bulk progress execution and CPU/GPU parallelism remain open. Their recommended order and semantic acceptance gates are in the [research review](research.md).
