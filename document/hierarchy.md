# Projected context and multilevel fragments

This stage follows [immutable preparation reuse](preparation.md), using `28c58fd` as its baseline. The [audit artifact](hierarchy.json) contains the exact source manifests, matching harness, paired samples, power observations, and a separate experiment that disables multilevel promotion while retaining context projection.

## What changed

A retained fragment now stores its own selected suffix. Replay supplies the current ancestor binding. This separates inherited data from the computation that actually depends on it.

The dependency key contains the anchor occurrence and only ancestor worlds that can appear in a remaining candidate domain. It retains their input positions for the existing distinctness and symmetry constraints. Ancestor token choices do not influence descendant enumeration; their exact identities are restored from the current cursor when a binding is delivered. An ancestor world that cannot enter any remaining candidate domain also cannot constrain this traversal. Presence filtering remains conservative: it does not assume that independently available inputs can pass a synchronized gate.

This permits the same fragment to serve different ancestor token combinations and irrelevant ancestor worlds. It preserves the complete binding presented to downstream synchronization, executable-read handling, and proof construction.

Joins can now retain fragments at several observed depths. Each layer owns its occurrence reverse index and a searched-domain interval. A changed searched domain invalidates the layers above it; an anchor or relevant ancestor retirement retracts only the dependent entries. Deeper valid fragments can help rebuild larger fragments. Changes to input order still reset this representation.

A completed child transcript can be appended to a recording parent in one operation. The parent remains private until traversal has actually finished its region. Partial reset discards it. Delivery still emits every original binding and pending step; copying a known transcript does not execute a gate or publish speculative evidence.

The representation is a bounded forest of retained fragments within one join. Parent transcripts currently copy projected child records rather than retaining persistent child links. Cross-plan whole-prefix sharing remains separate. This is not a general maintained relation DAG or an integration of direct fragments into exhaustive proof matching.

## Adaptive choice and bounds

Always using layers was slower. The first prototype added roughly 60% overhead on alternating-depth controls. Bulk transcript composition, context projection, and direct ownership of the active replay removed much of that cost, but long fragments still favored the single partition.

Promotion now requires observed cached fragments averaging at most 256 logical steps, in addition to the existing structural admission conditions. Long fragments retain the single partition. Short fragments in deep products can benefit from larger retained regions. This is a measured caching policy, not a restriction on program shape.

The existing local 4,096-record and global 65,536-record cache budgets remain. Each transcript also keeps the existing 65,536-step bound, including transcripts assembled from children. Keeping that bound avoids introducing unbounded reconstruction when an active replay is evicted. At most eight depths are retained per join; additional depths execute through the underlying cursor. Programs with more inputs remain supported.

Saturation, cold searches, incomplete recordings, changed order, and eviction all retain exact traversal. Logical retained-record accounting covers the smaller dependency keys, projected payloads, layer metadata, and live replay. Record counts describe retained logical structure rather than allocator bytes.

## Preservation argument

1. The descendant cursor consults inherited worlds only for world distinctness and same-pattern symmetry. It does not consult inherited token choices. The key retains every ancestor world that can enter a remaining presence-filtered domain, together with its input position.
2. Omitted ancestors cannot trigger either constraint in this region. Their complete slots are restored on replay, so later suffix matching and synchronized firing still receive their exact worlds, positions, and tokens.
3. A selected descendant belongs to a searched domain. Changes to such domains invalidate the containing layer, including negative results. Anchors and relevant ancestors additionally have occurrence reverse dependencies, processed before recycled sites are reused.
4. Projection is confined to a fixed input order, pattern, frame, and capture context. It does not merge proof histories or executable consumers.
5. Composition copies a complete, already known child transcript. The recording parent is never shared while incomplete. Reset and eviction therefore cannot expose undelivered future records as a completed parent.
6. Replay preserves every logical pending step. Eviction reconstructs at most one bounded active fragment, and saturation falls back without truncating a binding or changing the work budget.

## Paired native results

Measurements use `bazel run -c opt` on an Apple M5 Max, connected to AC power with idle sleep inhibited. Three rounds alternate baseline/current order with nine samples per workload per round. The table reports pooled medians of 27 samples per side.

| Workload | Before, ms | After, ms | Ratio |
| --- | ---: | ---: | ---: |
| Interior, width 8 | 6.1102 | 4.4791 | 1.364× |
| Interior, width 12 | 122.1661 | 92.3777 | 1.322× |
| Interior at depth 2 | 246.9371 | 153.7055 | 1.607× |
| Productive interior | 126.8548 | 97.1341 | 1.306× |
| Alternating depths, long fragments | 171.7654 | 79.5376 | 2.160× |
| Alternating depths, 4 ancestors, width 2 | 1.7009 | 0.2622 | 6.486× |
| Alternating depths, 8 ancestors, width 2 | 24.4547 | 2.0920 | 11.689× |
| Alternating depths, 12 ancestors, width 2 | 282.2257 | 30.5335 | 9.243× |
| Alternating depths, 6 ancestors, width 4 | 26.7101 | 4.4701 | 5.975× |
| Many ancestor token choices, negative suffix | 101.7885 | 32.1218 | 3.169× |
| Many ancestor token choices, productive suffix | 56.2510 | 34.4025 | 1.635× |
| Stable prefix | 3.5557 | 3.3540 | 1.060× |
| Changing prefix | 3.9271 | 3.7161 | 1.057× |
| Six factors of two | 66.8861 | 68.4100 | 0.978× |
| Decimal 1234567890 + 9876543210 | 203.2070 | 203.3735 | 0.999× |
| Decimal 12345 × 67890 | 226.3490 | 226.7920 | 0.998× |
| 4,096-world candidate domain | 12.3880 | 12.7883 | 0.969× |

The context workload drops peak logical retention from 4,292 to 211 records; its productive variant drops from 4,246 to 361 while preserving all 95,040 bindings and 2,938,344 logical steps. The 12-ancestor workload drops from 4,367 to 354 records and retains exactly 8,388,288 logical steps.

An additional three-round comparison isolates multilevel maintenance from projected single-partition caching. Long-fragment performance is equal within noise, as the cost gate retains the single partition. The four deeper cases improve a further 1.47×, 3.03×, 3.63×, and 1.44× with layers. The artifact records the isolated workspace and its one-line promotion guard explicitly.

All protected pooled results meet the predeclared 5% regression tolerance, or 10% for submillisecond cases. Arithmetic does not gain from this stage; six factors of two are about 2.3% slower in this audit. The substantial gains belong to repeated contextual matching and deeper products, not every program.

Reproduce the focused cases with:

```sh
bazel run -c opt //benchmark:partition -- --depth 8 --width 2 --count 2 --alternating --sample 9
bazel run -c opt //benchmark:partition -- --depth 1 --width 12 --alternating --sample 9
bazel run -c opt //benchmark:context -- --sample 9
bazel run -c opt //benchmark:context -- --productive --length 8 --sample 9
```

The shared maintenance harness constructs the index and initial join before timing. Timed work includes coherent updates, matching, and logical delivery. These focused measurements are not whole-program compilation or proof-reporting latency.

## Validation and ownership

The optimized full suite passes all 106 targets, including native/WASM conformance. The debug kernel suite passes all 182 tests. Six new tests compare layered traversal against the direct cursor across interruption offsets, projection with many token choices, overlapping domains, negative-to-positive changes, simultaneous deltas, recycled occurrences, captures, bounded storage, eviction, automatic activation, and more input depths than the cache retains.

The complete subscription mutation observations agree in all three paired rounds. Direct benchmarks preserve event/work or binding/work counts. All 19 exhaustive fixtures preserve state, event, work, record, and peak observations in ordinary and instrumented builds.

`joining/space.rs` determines relevant matching context. `dependency.rs` owns projected keys; `retention.rs` owns occurrence retraction; `layer.rs` owns one retained depth; `tree.rs` owns layered traversal and the active replay; `trace.rs` and `playback.rs` own transcript projection and delivery; `strategy.rs` owns measured admission. Initial depth sets and update requests have distinct argument shapes, and transcript projection uses an explicit range rather than an ambiguous extra count.

Residual rejection, exhaustive fragment integration, compact bulk execution, and CPU/GPU parallelism remain separate stages. This implementation neither adds dynamic rule syntax nor limits future plan registration to the initial catalog.
