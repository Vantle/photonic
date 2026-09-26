# Exhaustive replay dependencies

The existing completed transcript cache now includes each candidate's internal matching site as well as its public coherence position and immutable coherence identity. The cursor merges candidate streams by internal site. Removing and reinserting the same immutable coherences can recycle those sites in a different order without changing the old cache key. A completed transcript can then have a different pending/binding sequence from fresh enumeration.

The regression fixture selects sixteen A occurrences from seventeen, and sixteen B occurrences from sixteen. Its complete transcript fits the cache. Rebuilding the same state through removal and reinsertion reverses site assignment while preserving public locations and immutable coherence pointers. The test establishes that the fresh streams differ and that the changed index must not replay the old stream. It fails with the previous key and passes with the corrected key.

The additional key field is included in logical storage accounting. Existing cache and key limits remain unchanged; a key that no longer fits uses ordinary exact enumeration. This is a correctness fix, not a claim of broader replay coverage.

## Rule-sensitive extension experiment

A separate prototype removed the atom-only and coherence-only restrictions. Its key included exact candidate locations and sites, immutable coherence identities, the lexical chain of immutable context identities, the captured pattern and enumeration order. Weak references prevented retired allocation addresses from being reused while a key remained cached. Only complete transcripts were published, and eviction reconstructed the exact interrupted cursor.

The prototype passed checks for ancestor mutation, changed captures, context replacement, incomplete enumeration and eviction at every stream boundary. New production tests retain the context matching and exclusion checks; rule-sensitive completed replay remains disabled after the experiment's rejection.

The expanded [`transcript`](https://github.com/Vantle/photonic/blob/0daf296e1d126675ed445bfb7ed6674a379b1532/benchmark/transcript.rs) harness, removed after `0daf296`, measures atoms, coherence-owned rules and inherited context rules at widths 8, 32, 128 and 512. It includes unshared execution, fresh shared stores and reused shared stores, with a 100 ms warmup for every case. The optional `--source` argument to `//benchmark:runtime` measures a complete closed program using the existing runtime harness and default limits.

Three alternating rounds show 1.9–2.6× faster repeated coherence-rule matcher queries and about 1.2–1.3× faster repeated context-rule queries. Cold eligible rule queries are approximately 7–22% slower. The atom replay controls also incur additional key maintenance.

Complete-program fixtures combine two consumers of the same large rule-valued input with an independent eight-step stage chain. Both inputs and rule contents remain unchanged. The 16-occurrence fixture stays around 3.2 ms and the 24-occurrence fixture around 4.0 ms, without a repeatable complete-execution gain. Logical retained records rise from 4,323 to 5,124 and from 4,395 to 5,484 respectively. The cold 16-occurrence fixture rises from 187 to 276 records. All paired executions retain the same state, event and work counts; those counts alone are not a full proof-equivalence check.

The extension was removed. Matcher reuse in a synthetic query does not establish a runtime improvement. Enabling this feature still requires a complete workload that saves more than key construction, recording, retained state and invalidation cost. The dependency design and measured rejection are preserved in [transcript.json](transcript.json).

## Correction measurements

Three alternating rounds of the corrected key remain within the existing control tolerances. Warm atom replay changes from 0.384 to 0.400 ms at width 32, 1.582 to 1.619 ms at width 128, and 8.404 to 8.645 ms at width 512. Cold width-512 matching is essentially unchanged at 33.762 versus 33.825 ms. The complete-program controls are within 5% above one millisecond and 10% below it. Work counts agree for every transcript case; complete-program state, event, work, record and peak counts also agree. Rule-sensitive replay is disabled in both versions of this correction comparison.

## Validation

The final optimized suite passes all 111 Bazel test targets, including 259 runtime tests, the independent occurrence oracle, the pinned historical comparison, WebAssembly and headless Chrome. The [incremental occurrence audit](invalidation.md) records the main runtime changes and their separate timing and allocation matrix. The replay correction changes exhaustive cache dependency tracking; it does not change the direct-path indexing measurements in that audit.

```sh
bazel test -c opt //... //book:check
bazel run -c opt //benchmark:runtime -- --source /path/to/closed.wave
```

This benchmark was removed after `0daf296` and still runs at that commit:

- [`transcript`](https://github.com/Vantle/photonic/blob/0daf296e1d126675ed445bfb7ed6674a379b1532/benchmark/transcript.rs)
