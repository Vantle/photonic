# Indexed rule matching

The subsequent [canonical refinement phase](refinement.md) reduces elapsed time further while retaining the work and event counts below.

The native expression workload now takes 215,876 work tasks instead of 2,875,589 at commit `e20ca7b`: a 13.3-fold reduction. The 10,017 Photonic events and decoded results are unchanged for this workload. The implementation is general runtime code; it does not recognize numbers, operators, or evaluator labels.

This meets the order-of-magnitude work target. It does not meet the millisecond latency target. Matching task counts and CPU time measure different costs, and canonicalization setup remains significant. The results below distinguish those measurements rather than infer speedup from a smaller counter.

## Measurements

An isolated comparison of compiled executables used one warm-up and five samples per revision, alternating their order. The input, resource budgets, compact output, and final witness were identical. These are development-host measurements, not cross-platform performance guarantees. [Raw timings, phase measurements, input hash, platform, and reference results](indexing.json).

| Sample expression | Before (`e20ca7b`) | Indexed runtime |
| --- | ---: | ---: |
| Work tasks | 2,875,589 | 215,876 |
| Photonic events | 10,017 | 10,017 |
| Median compact CLI time | 3.050 s | 2.315 s |

The work reduction is 13.3×; elapsed time improves by about 24.1%. Neither the tenfold counter improvement nor the unchanged answer implies a tenfold latency improvement.

The separate in-process benchmark locates the remaining time:

| Phase | Median milliseconds |
| --- | ---: |
| Initialization | 1.311 |
| Direct execution | 2,231.562 |
| Compact summary | 0.001 |
| Releasing retained history | 5.283 |

Execution is the dominant remaining phase. Avoiding report rendering is useful, but additional output-format changes will not turn this result into a few milliseconds. Initialization is already measured in milliseconds. Full JSON trace encoding is excluded from these phase timings and remains a separate, substantial workload.

All 99 repository test targets pass. The existing 19 closed reference benchmarks retain their state and event counts; their raw indexed results are included for future comparisons. No claim of a universal timing improvement follows from this expression benchmark.

## Design

### Shared state index

Each state gets an immutable index when it is first searched. The index maps a frame and an exact term to a sorted list of worlds containing that term. Rule terms retain their captured frame; atom terms ignore capture, consistently with the original matcher. Separate frame lists support empty patterns.

A particle query obtains its posting lists, starts with the smallest one, and tests membership in the others. A missing posting or empty intersection proves that no world can match that particle. A complete joint query is impossible if any of its required particles has no candidate world. Such queries retain a completed empty cache entry, avoiding repeat setup and unnecessary scheduled searches or subscribers.

Posting lists only establish presence. The existing particle enumerator still selects concrete resource occurrences, enforces multiplicity, and handles repeated terms. The gate still preserves original operand positions, distinct-world requirements, and the symmetry restriction for equal patterns. The index never replaces a multiset with a set for rule application.

Search advances through candidate pairs in world/position order, using resumable cursors. Skipping impossible candidates removes unsuccessful work without reordering the successful world/position traversal within a search. Relative scheduling between searches can change because they finish sooner; complete semantic results and worker-count determinism remain the contracts. The expression fixture retains its existing event path.

Indexes are allocated lazily so states that are never searched do not pay construction cost. A state shares its index across its pattern searches. Index records, posting entries, and query candidate entries are included in record accounting. As before, these are soft record budgets, not hard byte or time bounds; index construction and query preparation occur inside coordinator work.

### Empty join intervals

The gate previously queued an Arrival or Follow task even when its known candidate interval was empty, including a terminal task after visiting the final candidate. It now queues only nonempty intervals.

This is not an absence premise in Photonic. It is an implementation check that an internal loop has no iterations. Candidate and prefix storage is retained, so later arrivals still join with previously stored data. Tests enumerate all 720 arrival orders of six slots for both equal and unequal two-position patterns, checking the complete binding set in 1,440 cases.

### Canonicalization without signature copies

CPU sampling of the indexed runtime identified substantial time in canonicalization setup, especially refinement classification. The previous classifier cloned structured signatures into a sorted map and then looked up each input signature again.

The new classifier sorts indices by borrowed signatures and assigns ascending ordinals to equal-signature groups. This produces exactly the same class ordinal for every input. It avoids copying the signature vectors and constructing the map. An exhaustive oracle checks 5,461 sequences of length zero through six over four values against independently ranked distinct keys. Existing canonicalization tests cover resource sharing, permutations, and exact reference configurations.

This changes implementation cost, not the graph equivalence relation. Captures, shared introductions, and private resources remain distinct wherever the semantics requires it. No probabilistic equality test, weaker canonical form, or label collapsing is introduced.

### State sharing and compact output

The direct path now reuses the immutable state through `Arc` when seeding its next runtime, avoiding a deep copy of a state it already owns. Full path history is still retained for reporting and cycle detection.

Compact CLI output now constructs only its final witness and summary counts. Previously it constructed every rendered state in the path even though the text output did not print them. Full JSON reports remain available with their complete state and event data. A test compares summary fields and witnesses to the full report for reached, initially reached, and unfinished paths.

## Research assessment

A trie is a useful storage technique, not a universal solution to matching cost. Leapfrog Triejoin demonstrates how ordered indexes can support worst-case-optimal relational joins up to logarithmic factors. Its guarantees apply to the relational query model and output-size bounds in that work; this runtime's resource projection and canonicalization are additional operations.[^trie]

Free Join explicitly addresses cases where traditional binary joins outperform worst-case-optimal joins, and unifies their planning and data structures. This supports starting with selective posting-list intersection for these short ground patterns, rather than assuming a full multiway trie engine will reduce latency on every workload.[^free]

CompactLTJ investigates the memory and execution-time tradeoffs of compact indexes for graph pattern queries. Its 2025 results reinforce that index representation and memory traffic matter alongside asymptotic bounds. Photonic's worlds are multisets with captured code and shared resources, rather than ordinary triples, so those benchmark results are not transferable speedup predictions.[^compact]

The current egglog implementation has moved to a database-oriented parallel backend, while its changelog also records subtle fixes involving incremental matching and secondary-index rebuilding. These are useful implementation references and examples of the validation burden. Adopting a database engine wholesale would not supply Photonic's source-projection or resource-identity semantics.[^egglog]

The implemented index is a selective exact-term intersection index. It is not advertised as a full Leapfrog Triejoin, Free Join, Rete network, or a proof of worst-case-optimal whole-program execution. It applies the relevant indexing principle while preserving the existing matching and application layers.

## Validation

The particle-enumeration oracle continues to check 5,440 cases, including repeated terms and rule captures. A new index oracle checks 65,280 combinations of patterns, frame choices, and multi-world states against a direct scan; this includes missing frames, empty patterns, rule captures, and atom capture insensitivity. The join-arrival and classification oracles cover the independent transformations described above.

The broader runtime suite checks source inference, generated code, captured environments, positive support, resource sharing, canonicalization, state/record suspension, pause/resume equivalence, and deterministic reports across worker counts. Native arithmetic and expression tests continue to check exact final configurations and cleanup. A work-budget regression test limits the complete sample to 250,000 work tasks and 10,200 events.

These bounded tests accompany local correctness arguments for the changes. They do not establish universal confluence or machine-checked soundness of the entire runtime. The new index consumes memory, and a fixed soft record budget can suspend at a different point because its actual retained structures are now accounted for.

## Reproduction

```sh
bazel test -c opt //... --test_output=errors
bazel build -c opt //mathematics/ternary:infix.program
bazel run -c opt //system:trace -- \
  "$(bazel info -c opt bazel-bin)/mathematics/ternary/infix.assembly.json" \
  "$PWD/mathematics/ternary/result.particle" --sample 5
```

The trace benchmark decodes its inputs before measurement and performs one warm-up. It separately measures initialization, direct execution, compact summary construction, and release of retained execution history. Durations are seconds. Source-program clones happen before each measured call. It uses the same explicit resource budgets as the expression recorder and requires a reached target; it does not encode full JSON traces during timing.

Compare compiled CLI executables separately when process startup and input decoding matter. Use the same assembled program, output format, budgets, warm-up, and rotated revision order. Full trace serialization and browser playback are different workloads and must not be substituted into a compact execution latency comparison.

## Remaining route to millisecond latency

The counter improvement makes further matching work easier to study, but it does not eliminate graph reconstruction, refinement, exact canonical ordering, flow projection, evidence management, or retained history. The initial CPU sample already showed why optimizing the largest task category alone would be insufficient: substantial canonicalization work happens during setup, outside its incremental task count.

The next major design question is incremental graph maintenance across a local transition. It requires transporting resource and frame identities, updating only affected graph signatures, and preserving the exact maps used by provenance. The direct-path runtime also still reseeds its per-runtime structures after every selected event. Reusing them safely requires precise dependency invalidation; it cannot be implemented by remembering label-only matches.

A smaller next step is to profile the final implementation over the complete run and compare execution, initialization, and history-release times. Candidate indexing has met its work target. The next latency experiment should follow those CPU measurements, with the current implementation and tests retained as the comparison baseline. Millisecond performance remains an engineering target, not a demonstrated result or a theoretical impossibility.

## Sources

[^trie]: Todd L. Veldhuizen. [Leapfrog Triejoin: A Worst-Case Optimal Join Algorithm](https://arxiv.org/abs/1210.0481). 2012 preprint, ICDT 2014.
[^free]: Yisu Remy Wang, Max Willsey, and Dan Suciu. [Free Join: Unifying Worst-Case Optimal and Traditional Joins](https://arxiv.org/abs/2301.10841). SIGMOD 2023.
[^compact]: [CompactLTJ: Space & Time Efficient Leapfrog Triejoin on Graph Databases](https://link.springer.com/article/10.1007/s00778-025-00945-5). The VLDB Journal, 2025.
[^egglog]: Egglog maintainers. [Changelog](https://github.com/egraphs-good/egglog/blob/main/CHANGELOG.md), including the 1.0.0 backend transition dated 2025-10-18 and subsequent matching/index fixes. Retrieved during this investigation; the branch is mutable.
