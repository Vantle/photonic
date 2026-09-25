# Documentation

The [webbook](../index.html) is the guide to the language, its verification, the standard library, proofs, the runtime and the repository. This directory holds its companions: the contracts it summarizes, the current runtime plan, and dated records of runtime work.

## Contracts and references

These describe the repository as it is and change with it.

| Document | Contents |
| --- | --- |
| [Standard library](../library/README.md) | Every package, request and answer, with the chain, vector and sort protocols. |
| [Runtime rule occurrences](occurrence.md) | Loading, ownership, consumption, matching locations and exact targets. |
| [Build organization](build.md) | Bazel packages, toolchains, caching and the lint policy. |
| [Continuous verification](automation.md) | The Buildkite jobs, their agent capacity and the fixtures tagged `memory`. |
| [Theorems](../theorem/README.md) | The proof contract, how to write a theorem, and the order in which the layers are proved. |
| [Program symmetry](symmetry.md) | Shapes, renamings and symmetries of programs: how the engine finds them, how to use it to connect fields, and what it finds in this repository. |
| [Program optimization learner](learning.md) | Correctness under every schedule, the objective and goals, edits, planning, inferring instead of checking, solving and proving optimality, learning loops and curriculum, training and results. |

## Plan

| Document | Contents |
| --- | --- |
| [Runtime performance roadmap](roadmap.md) | Priorities, acceptance gates and the semantic boundary for runtime work. |
| [Runtime review](review.md) | The latest review of the whole runtime and the changes it made. |

## Records

Each record describes the runtime at its baseline commit. Later records supersede its measurements, and code it names may since have moved or been deleted; its baseline commit shows that code. The JSON files beside the records hold their raw measurements. Records are listed from oldest to newest.

| Record | Last revised (UTC) |
| --- | --- |
| [Evaluation runtime](evaluation.md) | 2026-09-19 |
| [Persistent incremental evaluation](incremental.md) | 2026-09-20 |
| [Runtime optimization status](optimization.md) | 2026-09-20 |
| [Shared matching implementation](implementation.md) | 2026-09-20 |
| [Incremental matching and reachability](factorization.md) | 2026-09-20 |
| [Joined prefix reuse](prefix.md) | 2026-09-20 |
| [Shared joins and proof reuse](sharing.md) | 2026-09-20 |
| [Shared candidate domains](candidate.md) | 2026-09-20 |
| [Dispatch dependency maintenance](dispatch.md) | 2026-09-20 |
| [Joined-fragment maintenance](partition.md) | 2026-09-20 |
| [Interior matching and unfinished sharing](interior.md) | 2026-09-20 |
| [Immutable preparation reuse](preparation.md) | 2026-09-20 |
| [Projected context and multilevel fragments](hierarchy.md) | 2026-09-20 |
| [Capacity rejection before particle preparation](residual.md) | 2026-09-21 |
| [Immutable preparation across exhaustive queries](exhaustive.md) | 2026-09-21 |
| [Exact batching of cached waiting spans](batch.md) | 2026-09-21 |
| [CPU parallelism and the GPU decision](parallel.md) | 2026-09-21 |
| [Subscription reconciliation](reconciliation.md) | 2026-09-21 |
| [Architecture state](architecture.md) | 2026-09-21 |
| [Persistent binding payloads](segment.md) | 2026-09-21 |
| [CPU continuation](continuation.md) | 2026-09-21 |
| [CPU reuse and execution checkpoint](cpu.md) | 2026-09-21 |
| [Residual certificates and causal history](certificate.md) | 2026-09-21 |
| [Dispatch maintenance](maintenance.md) | 2026-09-22 |
| [Runtime lifecycle audit](lifecycle.md) | 2026-09-22 |
| [Canonical reporting maintenance](canonical.md) | 2026-09-22 |
| [Canonical resource storage](resource.md) | 2026-09-22 |
| [Shared report text](render.md) | 2026-09-22 |
| [Compact canonical resource mappings](mapping.md) | 2026-09-22 |
| [Shared canonical frames](preservation.md) | 2026-09-22 |
| [Compact subscription keys](registry.md) | 2026-09-22 |
| [CPU research before GPU execution](research.md) | 2026-09-22 |
| [Incremental report materialization](view.md) | 2026-09-22 |
| [Shared matching structure](plan.md) | 2026-09-22 |
| [Compiled scope membership](scope.md) | 2026-09-22 |
| [Join location boundary](location.md) | 2026-09-22 |
| [Subscription activation](activation.md) | 2026-09-22 |
| [Architectural opportunities](opportunity.md) | 2026-09-22 |
| [Demand-driven canonical refinement](normalization.md) | 2026-09-22 |
| [Demand-driven subscription admission](admission.md) | 2026-09-22 |
| [Exhaustive replay dependencies](transcript.md) | 2026-09-22 |
| [Incremental capture dependencies](dependency.md) | 2026-09-22 |
| [Incremental occurrence maintenance](invalidation.md) | 2026-09-22 |
| [Persistent occurrence populations](population.md) | 2026-09-22 |
| [Resource bounds from population changes](identity.md) | 2026-09-22 |
| [Shared incidence during canonical renaming](incidence.md) | 2026-09-22 |

## Archived and rejected

| Document | Status |
| --- | --- |
| [Dynamic rule construction](dynamic.md) | Archived proposal outside the runtime roadmap. |
| [Initial executable rules](loading.md) | Historical assessment, superseded by the occurrence contract. |
| [Subscription retention experiment](subscription.md) | Rejected prototype, removed from production. |
| [Empty search representation](dormancy.md) | Both prototypes rejected and removed; regression coverage retained. |
| [Refinement experiment checkpoint](experiment.md) | Both experiments removed; staged canonical ordering remains the baseline. |
