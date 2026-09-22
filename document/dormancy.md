# Empty search representation

Baseline: `72b8b94`. Status: both representation prototypes rejected and removed; regression coverage retained. The [audit artifact](dormancy.json) contains source snapshots, paired measurements, executable hashes, allocation observations, and verification logs.

## Hypothesis and boundary

The [subscription retention experiment](subscription.md) rejected eager maintenance of parked whole searches. This experiment investigates avoiding a full join representation while all candidate domains are empty, without retaining subscriptions after the existing removal boundary.

The earlier activation diagnostic recorded 47,532 all-empty admissions out of 66,492, including 43,773 single-domain empty admissions. Those historical counts motivated this experiment; they are not a timing attribution or proof that representation changes can save a proportional fraction of execution time.

An empty domain establishes that the current search cannot emit a binding. It is distinct from an empty input pattern, which can match an available coherence, and from a failed distinct-world assignment with nonempty domains, which may still expose pending steps. Only nonzero-arity searches with every domain empty and no live shared candidate subscription were eligible for compaction. Existing shared subscriptions retained their original representation and accounting.

The proposed compact state retained the immutable query, frame, and applicable shared stores. It omitted empty domain arrays, ordering, and traversal state. A relevant insertion restored the original empty join and applied the ordinary coherent update, preserving its ordering and strategy transition. The compact state charged the same logical records as the existing representation. An active query remained active if its domains later emptied; the experiment did not discard established caches or continuation state.

This still performed initial candidate discovery. It tested representation and initial join setup, not elimination of the candidate lookup work itself.

## Two layouts

The first layout boxed active joins to keep each subscription's representation compact. The second stored the active join inline, avoiding the added allocation while giving up most of the subscription-size reduction. Neither variant added a retained-query registry or changed rule contents, loading, or availability.

Each variant ran four complete expression workloads in three alternating processes per side, nine samples per process. All samples reached the expected target and preserved event counts, logical work, and reported path statistics. Timings use the current Apple M5 Max.

| Workload | Boxed baseline milliseconds | Boxed candidate milliseconds | Inline baseline milliseconds | Inline candidate milliseconds |
| --- | ---: | ---: | ---: | ---: |
| Addition | 5.924 | 6.052 | 5.918 | 5.836 |
| Three-factor product | 16.918 | 16.649 | 16.803 | 16.853 |
| Six-factor product | 58.065 | 62.184 | 57.817 | 57.575 |
| Nested expression | 22.199 | 21.805 | 22.011 | 21.824 |

The boxed six-factor result regresses about 7.1%, exceeding the existing 5% gate. Inline results range from about 1.4% faster to 0.3% slower. These small changes do not justify the extra state machine and delegation layer or meet the roadmap's 10% complete-execution target for substantial new machinery. The inline comparison removes the prominent regression, but it does not establish that allocation alone caused the boxed result.

The separate inline allocation diagnostic records median execution traffic of 83,563,509 requested bytes before and 83,569,685 after. Allocation calls change from 552,157 to 552,142, and phase peak requested storage from 27,581,524 to 27,582,852 bytes. Full-lifecycle observations agree and all tracked storage is released. These differences are negligible. This diagnostic supplies allocator counters, not independent latency evidence.

Both prototypes passed the kernel tests, including differential activation and shared-subscription checks. They were removed because of their cost and lack of measured benefit; neither received a full release-equivalence audit or was installed as a production option.

## Retained verification

The useful coverage now tests the existing [replay interface](../language/test/dormancy.rs) against a separately instantiated planned join. Nine input families and two captured owners run through 96 coherent mutations each. They cover empty configurations, one through three empty input slots, mixed empty and required inputs, repeated particles, wide particles, captured rule values, insertion into another frame, removal, and site reuse.

Partial traversal is followed by reset and complete delivery. Every skipped replay span is checked against individual pending steps from the reference join. Repeated eviction and activation must preserve the exact binding stream and completion. A separate case retains a shared candidate subscription across a transition to an empty domain and checks initial retained accounting against the planned join.

The final kernel suite has 233 passing tests. Production matching functions and representations are restored; the code changes retained from this work are test registration and coverage only. The rule-value migration remains a separate design assessment.

## Next selection

Do not add the compact search wrapper or repeat small layout variants without new evidence. Frequent empty admissions are not sufficient justification for another representation layer. Admission count, construction cost, and candidate lookup cost need to be distinguished before another lifecycle design is selected.

The larger opportunity remains avoiding repeated work across a layer, rather than maintaining extra state or merely moving it behind a wrapper. A future membership design must eliminate measurable construction or lookup work with one owner for invalidation and accounting. Shared maintained joins and lazy trace composition remain conditional on representative demand. This experiment supplies a stopping point for the tested representation approach, not a claim that runtime optimization is complete.
