# Demand-driven subscription admission

This increment follows [persistent occurrence populations](population.md). Its baseline is that implementation before owner-local consumer indexing and admission changes. The [raw audit](admission.json) contains the source patch, source and executable hashes, scripts, paired samples, allocation records, report comparisons, phase profiles, rejected trial and validation logs.

## Ownership and activation

Dispatch now groups actual context-owned rule occurrences at their physical owner. The group retains each occurrence's rule, capture owner and resource read. A request walks the current lexical ancestry and reads those local groups; world-owned rules still use the world reader index. Only owners reached by an enabled request materialize a group. Local population changes update cached groups by exact occurrence differences. Scope and lexical changes use the current ancestry during request construction, so descendants do not own copies of inherited groups. Eviction releases the groups and falls back to exact local indexed scans.

New subscriptions use the candidate domains already needed to construct their searches. If any input position has no candidate, construction stops and leaves no dormant search entry. Successful admission uses those same domains without a second lookup. Zero input positions remain executable without a coherence; an empty particle position requires an actual coherence. Existing searches remain maintained through later empty domains.

An absent subscription can activate when a local symbol arrives even if that symbol was already available elsewhere. This also applies when another capture owner already has a subscription for the same compiled input. An empty coherence can activate an empty particle input without changing any symbol. Dispatch selects affected inputs for both cases, and whole-context invalidation covers lexical and ownership changes. Resource identity, capture binding, evidence and exact report content remain unchanged.

## Validation

All 111 optimized Bazel test targets pass, including 273 runtime tests, native and browser checks. Formatting passes. Nine complete command reports compare exactly against the baseline, including work, state, event, capture and provenance. New tests cover deferred arrival under a different capture owner, empty coherence arrival with and without eviction, stable local membership through consumption and restoration, and owner retirement with slot reuse. The bounded transition oracle and other existing tests remain in the full suite.

The initial separate prerequisite check improved arithmetic but slowed wide consumption about 8–10% because viable searches performed candidate lookup twice. It was removed. The integrated admission check computes each domain once. Eager construction of all owner groups subsequently caused 10–53% slower initialization in the availability controls; demand-driven construction removed that regression. Both intermediate trials and their paired measurements are retained in the raw audit.

## Measurements

Optimized native binaries ran sequentially on the development Apple M5 Max without concurrent builds or tests. Three alternating paired rounds used 101 samples per small lifecycle, 51 per wide and availability control, 21 per expression and three per full streaming export. The table gives the median of the three round medians. Requested allocation bytes are separate from timing and are not RSS or WebAssembly heap measurements.

| Execution workload | Before | After | Change |
| --- | ---: | ---: | ---: |
| 64 ordinary transitions | 0.1394 ms | 0.1382 ms | 0.9% faster |
| 32 body entries and returns | 0.2162 ms | 0.1755 ms | 18.8% faster |
| 32 context consumptions | 0.1857 ms | 0.1912 ms | 3.0% slower |
| 256 staged context consumptions | 1.8580 ms | 1.9167 ms | 3.2% slower |
| `2+2` | 10.0162 ms | 8.9287 ms | 10.9% faster |
| `2*2*2*2*2*2` | 86.9067 ms | 74.5316 ms | 14.2% faster |
| `(2+2)*(2+2)` | 33.8281 ms | 29.8661 ms | 11.7% faster |

The three availability execution controls improve about 2–4%; their initialization changes by less than 1%. Complete wide-consumption lifecycle increases 0.7%. Complete six-factor streaming export changes from 2.947 to 2.936 seconds; its export phase remains about 2.843 seconds. The measured protected phases meet the existing 5% tolerance above one millisecond and 10% below it. The small consumption and wide execution regressions remain visible rather than folded into the arithmetic gain.

Six-factor execution allocation traffic falls from about 124.8 to 118.4 MB, but remains above the pre-population 102.4 MB reference. Wide execution allocation traffic is nearly flat at about 3.50 MB. Owner groups add about 67 KB of retained initialization data in the wide fixture. Full-export peak requested storage remains about 440 MB. Every measured lifecycle releases tracked allocations to zero.

A separate five-sample instrumented profile shows median dispatch time falling from 48.9 to 38.2 ms and subscription time from 31.0 to 25.0 ms. Successful search preparations fall from 79,191 to 11,781 in that diagnostic run. Index and rewrite remain about 15.9 and 10.2 ms, and matching about 3.1 ms. Nested phase times overlap and instrumentation affects timing; these figures identify remaining work, not a predicted speedup.

## Remaining work

Subscription admission is now cheaper, but dispatch still reconstructs request lists and tries candidates for unavailable local combinations. The runtime is not yet at the desired endpoint. Broad identity maintenance, arithmetic allocation traffic and complete-history reporting remain material. Further subscription work should target a measured repeated layer cost while preserving late activation and exact suspension; it should not retain every inactive search.

```sh
bazel test -c opt //... //book:check
bazel run -c opt //:format -- --check
bazel run -c opt //benchmark:expression -- '2*2*2*2*2*2' 2101 --sample 21
bazel run -c opt //benchmark:scope -- --width 256 --depth 16 --length 64 --sample 51
bazel run -c opt //benchmark:allocation -- expression '2*2*2*2*2*2' 2101 --sample 2 --export view --writer
```
