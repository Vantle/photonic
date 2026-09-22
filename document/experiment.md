# Refinement experiment checkpoint

The [staged canonical ordering](normalization.md) implementation remains the accepted baseline. Two subsequent experiments preserve canonical results in their runtime tests but fail to demonstrate a reporting improvement. Both have been removed from production and test code. Their source patches, binary hashes, measurements and validation logs are retained in the [experiment audit](experiment.json).

## Earlier symmetry

The first experiment applies exact world symmetry before requesting fine graph colors. Ordering refinement operates on equivalence-class representatives, and a group with one exact class requires no graph. Tests check refinement/quotient composition and canonical state, world, frame and resource mappings against the eager reference, including exact search step counts.

The six-factor streaming export still constructs and refines 9,707 graphs for 10,101 newly normalized states. Moving symmetry earlier does not remove any graph on this workload. A preliminary three-sample comparison increases the export phase from 2.771 to 3.022 seconds, about 9%. The instrumented run also spends about 0.25 seconds more in normalization. The earlier symmetry scan adds cost without solving the measured ambiguity, so this arrangement is rejected.

## Resumable refinement

The second experiment separates the partition refinement workspace from its execution. Canonical ordering supplies groups whose members need comparison. Refinement stops once every requested group has distinct colors, or when the partition reaches its fixed point. A later frame-ordering request resumes the same workspace.

This stopping rule is exact: every round only splits existing ordered partitions, preserving the relative order of vertices that already have different colors. A tied requested pair requires continued refinement to the fixed point. Existing exhaustive small-graph and generated larger-graph tests are extended to compare demanded pair ordering against an independent full-signature reference. A chain test verifies stopping after one round, resuming for another pair, ignoring empty and singleton demands, and idempotent completion. The canonical differential tests and all 277 experimental runtime tests pass.

Three sequential paired export comparisons produce these phase medians, in seconds. Each process measures a separate cold run followed by three fresh-instance samples. Builds and tests do not run concurrently with these measurements.

| Pair | Baseline | Experiment | Change |
| --- | ---: | ---: | ---: |
| Initial, baseline first | 2.815 | 3.110 | 10.5% slower |
| Repeat, experiment first | 2.808 | 2.846 | 1.3% slower |
| Repeat, baseline first | 2.816 | 2.813 | 0.1% faster |

The initial slowdown is not reproduced at the same magnitude; these preliminary results do not establish a stable 10% regression. They also show no material gain. The median of pair medians is about 1.1% slower. A separate one-sample instrumented comparison puts refinement including graph construction at 1.051 versus 1.037 seconds, while complete normalization changes from 1.873 to 1.887 seconds. Both versions still construct 9,707 graphs and take 30,303 canonicalization steps. Serialized byte counts and writer fingerprints agree in every measured sample. The resumable workspace and demand bookkeeping do not justify their complexity on this evidence.

## Retained state and next investigation

The accepted staged implementation is restored exactly, including its tests and explicit build source list. All 111 optimized Bazel test targets pass after restoration using cached results, including browser conformance, and formatting passes. These results validate the restored implementation; neither rejected experiment completed the full acceptance matrix or receives a production acceptance claim.

Expression reporting still spends substantial time building resource incidence and rebuilding the same resource relationships during renaming. A useful next investigation is a shared structural representation for those two operations, preserving their separate ordering responsibilities and exact identity mappings. That is a hypothesis requiring measurement, not an established speedup. Execution-side identity scans and arithmetic allocation traffic remain separate outstanding work in the [runtime roadmap](roadmap.md).
