# Shared incidence during canonical renaming

This increment follows [staged canonical ordering](normalization.md) and the [rejected refinement experiments](experiment.md). A canonical search now retains the resource graph it builds for refinement and uses that graph during renaming. The [raw audit](incidence.json) contains the source patch, executable and source hashes, paired measurements, allocation counters, phase profiles, exact-report comparisons and validation logs.

## Structure and ownership

Incidence construction records source resource identities in resource-vertex order. The existing graph already describes each resource's symbol, capture and occurrences in worlds, held populations and owned populations. Refinement retains this graph together with its world and frame colors. A boxed refinement keeps the larger scratch object outside each history record.

When an ordering requires refinement, renaming projects resource adjacency into canonical world and frame positions. It sorts each resource's membership sequence and orders resources by the same symbol, capture, membership and source-identity key as direct renaming. Duplicate membership edges remain significant. The resource identities provide the same final tie breaker and source-to-canonical mapping.

Both renaming paths use one state assembly component. This component constructs the frame mapping, invokes a resource-ordering producer and applies the resulting identities to worlds, owned particles, held particles and captures. Direct renaming remains available when leading ordering keys resolve the state without a refinement graph. Initial-state construction also uses direct renaming.

The graph belongs to one canonical search over an immutable source state. It remains available across world/frame permutations and suspended search steps, then releases with the search. There is no additional cross-state cache or invalidation policy. Completed canonical states retain their existing mappings and ownership.

## Validation

All 111 optimized Bazel test targets pass, including 277 runtime tests and native/browser conformance. Formatting passes. Nine complete command reports compare exactly with the baseline, without removing fields. Every lifecycle observation, streaming byte count and writer fingerprint agrees. Twelve symmetry controls retain their exact step counts and completion status; repeated inspection retains its observations.

The generated canonical differential test now compares graph-based renaming directly against token-based renaming for forward and reversed world/frame orders, before checking the complete canonical search against its eager reference. The 256 generated states and their reversed world orders cover shared identities, captures, held and owned resources, empty states and unreachable frames. A dedicated test covers repeated membership within one world, membership across every owner kind, equal resource keys requiring an identity tie break, sparse identifiers up to `usize::MAX`, captured frames and unreachable storage. Existing suspension, resumption, collision and bounded transition-oracle checks remain in the full suite.

## Measurements

Optimized native executables ran sequentially on the development Apple M5 Max, without concurrent builds or tests. Three alternating rounds use 101 samples for small lifecycles, 51 for wide consumption and availability, 21 for compact expressions and three for full streaming export. The table reports medians of the round medians. The existing limits remain 5% above one millisecond and 10% below it. Every protected phase and control passes.

| Workload or phase | Before | After | Change |
| --- | ---: | ---: | ---: |
| Six-factor streaming export | 2.842 s | 2.349 s | 17.3% faster |
| Complete six-factor streaming lifecycle | 2.936 s | 2.442 s | 16.8% faster |
| Compact six-factor execution | 74.949 ms | 74.669 ms | Approximately flat |
| Wide-consumption execution | 1.875 ms | 1.896 ms | 1.1% slower |
| Complete wide-consumption lifecycle | 14.248 ms | 14.291 ms | Approximately flat |
| Repeated inspection | 15.111 ms | 15.213 ms | 0.7% slower |

Small lifecycle totals increase about 1–3%. Compact expression initialization increases about 3.7–4.4%, within the existing limit. These are measured costs, and this increment establishes a reporting improvement rather than a general execution speedup. Symmetric ring controls with eight and thirty worlds improve about 28% and 37%, respectively, at the same bounded 1,000-step search limit; both remain incomplete at that limit.

Six-factor cumulative requested allocation falls from 12.964 to 10.197 GB, a 21.3% reduction. Peak requested storage changes from 440.37 to 439.70 MB, approximately flat. Wide-consumption allocation remains about 70.2 MB. Every measured lifecycle returns tracked retained allocations to zero. These counters measure requested Rust heap bytes, not RSS or WebAssembly heap size.

The separate final phase profile attributes the gain to renaming: its aggregate time falls from about 746 to 292 ms, while complete normalization falls from 1.878 to 1.425 seconds. Refinement including incidence remains about 1.06 seconds, and rendering remains about 162 ms. Both versions construct 9,707 graphs for 10,101 normalized states and perform 30,303 canonicalization steps. Nested phase timers overlap.

## Interface experiment and remaining work

The first prototype passed a completed frame mapping and resource mapping into the common assembly function. Its export improvement was substantial, but wide-consumption execution slowed about 9% and several initialization phases exceeded their limits. Instrumented execution did not reproduce that slowdown, so the phase profile did not establish its cause. The final interface owns frame-mapping construction and accepts a resource-ordering producer, allowing each path to compose its ordering with the common assembly operation. The full final matrix passes; the rejected prototype and its measurements remain in the audit.

The reporting benchmark also exposes execution and initialization phase profiles for ordinary lifecycle runs. These diagnostic timers compile out of normal binaries and remain separate from timing acceptance.

Graph construction and refinement still consume about a second in the six-factor export. Execution-side identity scans and arithmetic allocation traffic also remain open. This increment removes duplicated resource-incidence reconstruction during renaming; it does not establish completion of the broader runtime roadmap.

```sh
bazel test -c opt //... //book:check --test_output=errors
bazel run -c opt //:format -- --check
bazel run -c opt //benchmark:lifecycle -- expression '2*2*2*2*2*2' 2101 --sample 3 --export view --writer
bazel run -c opt //benchmark:allocation -- expression '2*2*2*2*2*2' 2101 --sample 2 --export view --writer
bazel run -c opt //benchmark:symmetry
```

This benchmark was removed after `0daf296` and still runs at that commit:

- [`reporting`](https://github.com/Vantle/photonic/blob/0daf296e1d126675ed445bfb7ed6674a379b1532/benchmark/lifetime.rs) with `expression '2*2*2*2*2*2' 2101 --sample 3 --export view --writer`
