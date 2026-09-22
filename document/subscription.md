# Subscription retention experiment

Baseline: `f477c80`. Status: rejected prototype, removed from production. The [audit artifact](subscription.json) contains the diagnostic and prototype patches, executable hashes, complete timing samples, lifecycle simulations, source restoration hashes, and reproduction drivers. Production runtime semantics and rule definitions are unchanged.

## Question

The [activation audit](activation.md) found 66,492 admissions in the six-factor expression, including many short-lived queries removed while their input was globally disabled. Repeated numeric coordinates alone did not prove reusable context or justify retaining searches. This experiment tests the next question: after conservative context invalidation, can bounded retention save more construction than it adds in maintenance?

## Lifecycle model

A separate diagnostic records admissions, removals, visits, frame reconciliation, and input dependencies touched by each coherent update. It runs addition, a three-factor product, a six-factor product, and a nested expression. Each warmup and measured run produces the same lifecycle event sequence after sorting the unordered per-frame touch notifications within each update. Only those notifications are normalized; application and subscription event order remains intact. Logging makes these diagnostic durations unsuitable for performance conclusions.

The offline policy parks a removed subscription only when its frame remains present, its input is globally disabled, and reconciliation is partial. A full frame reconciliation invalidates every parked key for that frame. Reappearance of the same frame/input/owner key counts as a potential hit only before that invalidation. The policy evicts the oldest parked entry when its count limit is exceeded. The simulation also evaluates retaining only subscriptions never visited during their active lifetime.

These are potential reuse counts, not a proof that old results remain valid. The model explicitly counts further dependency-triggered maintenance while subscriptions are parked. It includes maintenance just before full invalidation and the update at parking, matching the prototype's eager approach. A more selective implementation could avoid some of that work. Recorded weights use the subscription's charge at removal; they are occupancy proxies, not a simulated byte bound or exact retained-record peak as domains change.

At a capacity of 128 entries, retaining all eligible subscriptions gives:

| Workload | Admissions during execution | Potential hits | Additional maintenance calls | Maintenance per hit |
| --- | ---: | ---: | ---: | ---: |
| Addition | 4,584 | 2,836 | 42,663 | 15.0 |
| Three-factor product | 16,212 | 10,717 | 130,435 | 12.2 |
| Six-factor product | 66,492 | 48,764 | 426,099 | 8.7 |
| Nested expression | 21,222 | 14,939 | 171,519 | 11.5 |

The median gap from removal to reuse is nine generations in all four cases. For the six-factor case, 90% of hits return within 34 generations and 99% within 114. A capacity of eight captures only 836 hits; 32 captures 18,109. An effectively unbounded policy captures 51,094 but introduces 1,009,838 maintenance calls. Restricting the 128-entry policy to previously unvisited subscriptions still creates 380,144 maintenance calls for 44,885 hits. None of these counts is a speedup estimate.

The six-factor trace includes 53,308 eligible removals when initial subscriptions are included; the earlier 53,307 count concerned execution admissions. This difference in population is intentional.

## Prototype

The prototype adds a separate ordered store capped at 128 parked entries. It removes entries from the active registry normally, then parks eligible queries. Every affected frame updates parked queries whose input dependencies may have changed. Full frame reconciliation drops its parked entries. Activation installs the current consumer list and reuses the maintained search. Cache eviction drops parked state, and retained accounting includes each parked entry plus its metadata.

This is deliberately an experiment with whole searches, not the proposed final architecture. Its second registry duplicates lifecycle responsibilities. An entry-count bound is also insufficient as a general memory policy when individual domains can be large. A production design would need a storage budget, unified ownership, and broader tests of capture, reuse, eviction, and suspension.

The prototype is timed without diagnostic logging. Each side runs in three alternating processes, nine samples per process, on the current Apple M5 Max. Every sample reaches the expected target. Event counts, work counts, and all reported path statistics agree across both sides. Those checks establish comparable observed workloads; they are not a full semantic equivalence proof for the rejected implementation.

| Workload | Baseline milliseconds | Prototype milliseconds | Regression |
| --- | ---: | ---: | ---: |
| Addition | 6.018 | 11.274 | 87.3% |
| Three-factor product | 17.092 | 32.981 | 93.0% |
| Six-factor product | 57.865 | 105.673 | 82.6% |
| Nested expression | 22.099 | 41.914 | 89.7% |

The experiment fails both the regression gate and the roadmap's target of at least 10% lower complete execution latency for substantial reuse machinery. It was removed immediately. The two modified production files were restored byte-for-byte, and Bazel rebuilt the expression and profile targets. The restored expression executable's hash equals the saved baseline hash. No prototype path, feature flag, or dormant compatibility code remains.

## Architectural consequence

Do not pursue whole-search parking with eager maintenance as the next optimization. A high admission hit rate is insufficient when queries spend multiple generations inactive and their dependencies keep changing. This result rejects the measured policy; it does not prove every possible retention or maintenance policy is slower.

The next candidate should separate immutable membership information from materialized candidate domains and enumeration state. A disabled membership should not require a private domain update on every potentially relevant mutation. Candidate data must either be reconstructed when demanded or validated through a shared, exact invalidation mechanism. Merely keeping numeric keys or marking data dirty without a safe reconstruction path is insufficient.

Before implementing that design, measure the remaining admission cost by input arity and by whether all candidate domains are empty. Determine how much is query construction versus candidate lookup and retained storage. Use that evidence to decide whether a compact inactive representation, compiled membership template, or shared domain layer can meet the complete-program gate. Preserve logical accounting, consumer identity, captures, occurrence generations, and interrupted delivery. The [rule-loading assessment](loading.md) remains a separate, unimplemented semantic proposal.
