# Demand-driven canonical refinement

This increment follows [subscription admission](admission.md). It stages canonical ordering so graph refinement is computed only when an ordering group needs it. The [raw audit](normalization.json) contains the source patch, source and executable hashes, reproduction scripts, paired samples, allocation diagnostics, phase profiles, exact reports and validation logs.

## Ordering contract

World ordering first groups by the existing parent scope chain and particle symbols. Frame ordering first groups by the existing scope chain, occupied world positions and captured world occurrences. These are the same leading components of the previous ordering keys. Only a group with more than one member requests the next key: its full graph refinement color. A singleton's position is already determined by its leading key, so its refinement color cannot change the ordering.

One canonical search owns a lazily initialized refinement result shared by world and frame ordering. If either needs graph colors, the existing incidence construction and complete refinement algorithm run unchanged. Symmetry quotienting is also requested only when an ordering group remains ambiguous. It preserves the same representative classes and permutation order. There is no sharing between canonical searches and no new invalidation or eviction policy.

This preserves the existing lexicographic ordering exactly. Splitting each leading-key group by its refinement color yields the same groups as sorting by the complete tuple. Unique groups need no refinement value. Canonical states, resource mappings, world and frame mappings, search steps and serialized reports remain unchanged.

## Validation

All 111 optimized Bazel test targets pass, including 276 runtime tests, native and browser conformance. Formatting passes. Nine complete command reports compare exactly with the baseline. Twelve symmetry controls retain their exact completion and canonicalization step counts, and the repeated-inspection control retains its observations.

New ordering tests compare staged keys with complete tuple keys across generated partitions and verify that singleton groups never call the refinement or equivalence provider. A separate eager canonical reference checks complete state and identity mappings and exact step counts over 256 generated states and their reversed world orders. Cases include shared resources, captures, held and owned occurrences, empty states and unreachable frames. Dedicated tests exercise world ambiguity, frame-only ambiguity and complete normalization without constructing a refinement graph. Existing partition, collision, report, suspension and bounded operational-oracle tests remain in the full suite.

## Measurements

Optimized native binaries ran sequentially on the development Apple M5 Max, without concurrent builds or tests. Three alternating rounds used 101 samples per small lifecycle, 51 per wide and availability control, 21 per compact expression and three per full streaming export. Figures below are medians of the three round medians. Symmetry controls use their established warmup and 25 samples in each of three alternating rounds; inspection uses 21 samples per round. The original 5% tolerance above one millisecond and 10% below it remains unchanged. Every protected phase and control meets its tolerance.

| Workload | Before | After | Change |
| --- | ---: | ---: | ---: |
| Ordinary history reporting | 0.640 ms | 0.349 ms | 45.5% faster |
| Scope history reporting | 0.394 ms | 0.220 ms | 44.1% faster |
| Small consumption reporting | 0.206 ms | 0.118 ms | 42.5% faster |
| Wide consumption reporting | 13.990 ms | 7.894 ms | 43.6% faster |
| Complete wide consumption lifecycle | 20.198 ms | 13.965 ms | 30.9% faster |
| Six-factor streaming export phase | 2.845 s | 2.788 s | 2.0% faster |
| Complete six-factor streaming lifecycle | 2.937 s | 2.878 s | 2.0% faster |

Compact six-factor execution is approximately flat, at 73.937 versus 73.389 ms. The ordinary execution control improves about 8.5%, and repeated inspection about 9%. Symmetric controls are mixed: the largest increase in the aggregate medians is about 6.8% for a three-microsecond case, within its existing submillisecond tolerance. These are workload-specific gains, not a general expression-export speedup.

Wide lifecycle allocation traffic falls from 108.2 to 70.2 MB; ordinary lifecycle traffic falls from 5.71 to 3.66 MB. Six-factor streaming lifecycle traffic falls only from 13.149 to 12.964 GB. Its peak requested storage increases about 0.13 MB, from 440.24 to 440.37 MB. Every measured lifecycle releases tracked allocations to zero. Requested allocation bytes are not RSS or WebAssembly heap measurements.

## Attribution and remaining work

The new reporting profile separates complete normalization, incidence construction, graph refinement, resource renaming and node rendering. Before this change, a three-sample instrumented six-factor export spends about 1.85 of 2.73 seconds in normalization. Incidence construction accounts for about 0.54 seconds, refinement including incidence about 1.06 seconds, and renaming about 0.72 seconds. Rendering accounts for about 0.16 seconds. Nested phase measurements overlap and are separate from timing acceptance.

Demand-driven ordering avoids the graph on ordinary and wide-consumption histories. It still needs refinement for 9,707 of the six-factor history's 10,101 newly normalized states, explaining that workload's modest gain. Its graph construction, refinement and renaming costs remain substantial. The next reporting work needs to reduce those repeated costs while preserving exact canonical identities; merely removing node rendering cannot address most of the measured latency.

```sh
bazel test -c opt //... //toolchain/browser:check
bazel run -c opt //:format -- --check
bazel run -c opt //benchmark:reporting -- expression '2*2*2*2*2*2' 2101 --sample 3 --export view --writer
bazel run -c opt //benchmark:lifecycle -- expression '2*2*2*2*2*2' 2101 --sample 3 --export view --writer
bazel run -c opt //benchmark:allocation -- expression '2*2*2*2*2*2' 2101 --sample 2 --export view --writer
bazel run -c opt //benchmark:symmetry
```
