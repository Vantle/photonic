# Join location boundary

Baseline: `0361a20`. The [audit artifact](location.json) contains executable and source hashes, three alternating timing rounds, allocation observations, exact-report comparisons, the test logs, and the benchmark driver.

The join engine previously stored stable site identity in `crate::slot::Slot.world`, then overwrote it with a current world ordinal at delivery. Both meanings had the same type. The inspected implementation performed that conversion correctly, but a cursor or transcript could accept a delivered slot without a compiler error. More aggressive subscription reuse would make that distinction increasingly important.

The join engine now owns a private `joining::slot::Slot` with an explicit `site` field. Cursors, stored traces, prefix composition, playback, and dependency extraction use this type. `Join::step` constructs the existing delivered slot type by resolving each site through the current index. Token vectors move across the boundary; the conversion adds neither token cloning nor unsafe reinterpretation. The compiler rejects passing a delivered slot directly into internal trace or cursor operations.

This is a representation boundary, not an algorithm change or a fix for a demonstrated incorrect result. Raw index arguments still use integer identifiers, and generation validation remains the responsibility of existing index and invalidation logic. The change does not make arbitrary cached bindings safe to reuse after site recycling. Subscription discovery and domain lifecycle maintenance remain unfinished.

## Verification

All 110 Bazel test targets pass, with 106 executed and four cached. The final independent world-location fixture was refined afterward and the kernel target rerun: all 231 kernel tests pass. Benchmark and command targets build through Bazel; the suite includes the browser test.

The new location test derives expected world combinations directly from current state. It rotates coherences through 96 mutations in each of four configurations, varies narrow and wide particles, places matching candidates on both sides of the 64-site boundary, and checks partial traversal, reset, eviction, token ownership, exact delivery completeness, and retained accounting. Existing tests continue to cover shared, partitioned, layered, capture-sensitive, and replay behavior.

All 82 protected CLI reports remain byte-identical, including 60 empty-input reports at different work budgets. The 11 rule-loading observations used by the separate [design assessment](loading.md) are also unchanged. No rule definition, loading behavior, scope contract, output, or target interpretation changes.

## Performance

Three alternating processes per side, nine samples per process, on the current Apple M5 Max. Every workload preserves its non-timing observations. All 11 comparisons pass the existing 5% execution regression tolerance for these millisecond-scale cases.

| Workload | Baseline milliseconds | Candidate milliseconds |
| --- | ---: | ---: |
| Addition | 5.727 | 5.828 |
| Three-factor product | 16.341 | 16.445 |
| Six-factor product | 56.189 | 56.750 |
| Decimal addition | 171.643 | 172.272 |
| Decimal multiplication | 192.416 | 191.693 |
| Direct join | 2.118 | 2.057 |
| Partitioned join | 2.251 | 2.174 |
| Layered join | 2.434 | 2.424 |
| Productive mutation | 4.101 | 4.109 |
| Alternating mutation | 1.220 | 1.275 |
| Scalar mutation | 4.527 | 4.506 |

The alternating-mutation case is about 4.5% slower in this run; it passes the gate but is not evidence of a speedup. Median execution allocation traffic is unchanged at 83,566,749 requested bytes and 552,158 allocations for the six-factor full-lifecycle diagnostic. Median full-lifecycle peak requested storage is unchanged at 372,229,617 bytes, and every measured run releases all tracked storage. These counters describe native allocator requests, not process memory or a guarantee about other compilers and machines.

The reason to retain this change is the explicit, compiler-enforced boundary between internal sites and delivered world positions. No overall performance improvement is claimed.
