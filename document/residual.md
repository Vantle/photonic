# Capacity rejection before particle preparation

This stage follows the [multilevel audit](hierarchy.md). Its baseline is `6763094`, including immutable preparation sharing and projected multilevel fragments. The change applies to compiled direct matching; exhaustive search retains its existing preparation and progress behavior.

## Implementation

Before preparing a particle match, reject a world with fewer tokens than required input positions. For a repeated term, reject when its available token count is smaller than its required multiplicity. Worlds of at most sixteen tokens use a short scan that stops once the required count is reached; larger worlds use the existing counted index. Patterns with no repeated terms avoid multiplicity lookups. The compiled group's count and the pattern width already establish whether a term repeats, so no duplicate summary field is stored.

Rejected cursors share one immutable, lazily initialized failed preparation. Their private selection is empty. This eliminates repeated allocation for failure while retaining the existing cursor representation and positive matching path. The shared preparation is initialized once per process; it contains no pattern, token, world or capture identity.

Candidate membership and ordering do not change. The candidate is still visited, and its match cursor still returns the same failure at that visit. This is earlier implementation rejection, not pruning of logical traversal steps or a new physical join order. It does not yet implement residual distinct-world assignment filtering across several inputs.

## Semantic argument

Every successful binding selects one distinct token identifier for each input position. A world with insufficient total tokens cannot supply it. A repeated requirement needs at least its multiplicity of matching tokens; counting duplicate identifiers can only overestimate capacity. Counts therefore cannot reject a valid match. Passing the count still falls through to exact identifier deduplication and combination enumeration.

Term matching retains executable-rule capture identity. Empty patterns continue through normal preparation and still emit their empty binding. A failed shared preparation is always nonviable, including after reset, and cannot emit a binding. Candidate domains remain presence filters, so failed logical visits and their surrounding scheduling remain intact.

Smaller failed cursors reduce reported retained memory in direct matching. Record limits still count retained structures and retain their existing enforcement; this stage does not lower charges for memory that is still retained. Work budgets, delivery steps, executable reads, bindings and mutation order are compared separately from that intentional footprint reduction. Exhaustive record and peak observations are required to remain unchanged.

## Acceptance

The audit compares native `bazel run -c opt` executions with the same harness on both revisions, alternating run order under AC power. It includes the protected matching and arithmetic matrix, exhaustive fixtures, repeated negative preparation and a productive control. The [raw audit](residual.json) records all samples, source hashes, power checks and baseline harness identity. Three alternating rounds used nine samples per parameterized case; fixed exhaustive controls used their existing twenty-five-sample harness.

An initial representation using an optional preparation pointer was rejected after a large stable-join regression. The accepted representation must pass the same wider gates; a faster isolated failed match is insufficient.

| Workload | Before | After | Speedup |
| --- | ---: | ---: | ---: |
| 32,768 failed preparations, width 3 | 1.715 ms | 0.249 ms | 6.89× |
| 32,768 failed preparations, width 8 | 0.954 ms | 0.277 ms | 3.44× |
| 32,768 failed preparations, width 32 | 0.993 ms | 0.651 ms | 1.53× |
| 32,768 successful preparations, width 3 | 2.176 ms | 2.211 ms | 0.98× |
| Repeated shared preparation | 0.243 ms | 0.208 ms | 1.17× |
| Six factors of two | 63.190 ms | 63.222 ms | 1.00× |
| Ten-digit addition | 189.575 ms | 190.060 ms | 1.00× |

These are pooled medians within this paired audit, not comparisons with timings from another thermal session. Every protected workload passed the declared five-percent gate, with ten percent allowed for submillisecond measurements. The result improves repeated rejection rather than delivering a new arithmetic speedup.

All serialized subscription steps, reads, bindings, work counts, preparation counts, reuse counts and eviction observations agree across revisions. Only retained memory decreases. All nineteen exhaustive fixtures retain identical state, event, work, record and peak observations in both ordinary and instrumented builds. The kernel adds randomized comparison against the original particle matcher across empty and repeated patterns, captures, duplicate identifiers, small and indexed worlds, and reset.

The next stage is bounded immutable preparation reuse in exhaustive matching. Multi-input gate state, source inference and consumer-specific proof projection must remain private. General residual assignment decomposition and changed traversal order remain outside this implementation.

Final validation passed all 106 optimized Bazel test targets, including native/WASM conformance, all 183 debug kernel tests, formatting and diff checks. An earlier suite hit thirteen wall-clock timeouts, including tests that had printed passing results. The unchanged source passed the full suite under `caffeinate -i`; the interrupted run is retained in the audit notes rather than counted as a pass.
