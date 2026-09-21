# Persistent binding payloads

Baseline: `7b60f1f`. Immutable leaf payloads and parent-prefix links replace copied binding payloads in retained join transcripts. Record arrays, publication frontiers and consumer playback remain private. A parent retains only immutable selections, not the child's dependency registration or mutable cursor. This is a DAG of binding payloads; record headers are still copied.

Accounting conservatively charges each represented selection and every parent link, even when physical storage is shared. Existing local and global bounds still control admission; refusal resumes the exact traversal. Tests compare every poll against direct enumeration, including mutation, eviction, capture and unfinished publication. New lifetime and refusal checks verify reconstruction with the current consumer prefix after dropping the original child, waiting coalescence, seeking and budget release. All 197 debug kernel tests pass.

The [raw measurements](segment.json) include a three-round native matrix and the focused `bazel run -c opt //benchmark:segment` payload benchmark. The latter copies or extends 32,768 traces per sample, with 9 samples after warmup. Snapshot duplication improves 3.2–56.4×; parent composition improves 1.1–10.8× as width/token payload grows. These are storage-operation gains, not end-to-end arithmetic claims. Expression, addition and multiplication are within about 1.3% of baseline; existing productive/context fixtures are flat.

The intermediate audit found: the repeated negative-preparation control regresses approximately 13% and one sustained control approximately 5%. The raw matrix retains these results. This representation is not evidence that every CPU roadmap item is complete, nor a reason to conceal a protected regression.

The [final CPU audit](cpu.md) resolves these integrated performance regressions within the declared tolerances. Final payload measurements retain 3.3–55.9× snapshot gains and 1.1–10.8× parent-composition gains; the full suite now contains 107 Bazel targets and 201 debug kernel tests.
