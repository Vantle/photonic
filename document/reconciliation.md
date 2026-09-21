# Subscription reconciliation

Baseline: `8c68ad69196f77c2601d294ac5c23b13505f2175`. The [raw audit](reconciliation.json) records source hashes, arguments, all samples and sleep-interruption checks. Measurements use optimized native Bazel executables on an AC-powered Apple M5 Max, alternating baseline and candidate over three rounds without competing builds.

Subscriptions now live in frame-local ordered registries. Reconciliation merges sorted requested keys with existing keys, extracting obsolete entries directly. This removes repeated binary searches, temporary removal lists and duplicate frame counts. Consumer ordering and selected-input filtering are unchanged. Subscription reconciliation owns a separate module; registry accounting belongs to its container.

A lazily built reverse lexical graph maintains parent-to-child edges through coherent changes. Descendant discovery visits the affected region instead of scanning every active frame. Small states retain the original scan. The cache includes inactive ancestors, handles replacement and resizing, and is discarded under record pressure with an exact scan fallback. Its additional slots and edges are charged as retained records. This is an acceleration cache, not a language depth limit or an execution order.

The empty impossible particle matcher also constructs its known empty cursor directly instead of invoking generic selection initialization. It still owns the same immutable preparation and returns the same progress.

| Protected workload | Before | After |
| --- | ---: | ---: |
| Six factors of two | 66.24 ms | 65.21 ms |
| Ten-digit decimal addition | 201.12 ms | 201.91 ms |
| Five-digit decimal multiplication | 223.21 ms | 222.61 ms |
| Repeated negative preparation, 1,048,576 visits | 10.01 ms | 9.09 ms |

The 4,096-frame lexical-update fixture improves about 9×; the 256-frame fixture improves about 2.4×. These are focused maintenance gains, not arithmetic speedups. The 16-frame scan control is approximately flat. The final matrix stays within its stated 5% protected-workload and 10% submillisecond tolerances when samples are pooled across rounds. Individual rounds contain larger outliers; all are retained. The submillisecond shared-preparation fixture regresses about 8%. Longer calibrated negative and small exhaustive controls avoid drawing conclusions from very short process-to-process timings.

Rejected experiments include a larger membership promotion threshold, eager root-pattern capture preparation, a tiny-frame occurrence scan, and inlining candidate/context lookup boundaries. Some improved one fixture while regressing another; none is included.

Validation: all 106 optimized Bazel test targets pass, including native/WebAssembly conformance; all 195 debug kernel tests pass; formatting and whitespace checks pass. New differential tests compare 4,096 registry mutations with an ordinary ordered map and cached lexical discovery with the original scan through reparenting, resizing and eviction. Runtime and proof fixtures preserve exact state, event, work and record observations. Subscription fixtures preserve all non-footprint observations; the intentionally added lexical cache changes retained-record counts. Tests establish these checked cases, not a universal proof of semantic equivalence.

Persistent binding storage, broader exhaustive joined reuse, residual progress certificates, contextual flow reuse and large-task parallel admission remain separate stages.
