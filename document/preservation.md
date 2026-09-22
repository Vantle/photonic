# Shared canonical frames

Baseline: `a0cd0ed`. The [compact mapping audit](mapping.md) reduced retained engine storage to approximately 150 MB in the six-factor full export. This increment shares equal immutable frames across newly materialized canonical states in one direct-path report. Rules, programs, canonical identities, work accounting and serialized observations remain unchanged.

## Ownership

A private canonical storage component owns an ordered set of frame handles. While `Search::report` initializes a missing canonical form, this component replaces equal frame handles with an existing shared handle. Equality includes scope, parent, lexical reference, the ordered held-token sequence, resource identifiers, symbols and captures. Frame positions and all world/frame/resource mappings remain separate and unchanged; sharing allocation does not identify distinct logical occurrences.

The table belongs to one reporting call and is destroyed before that call returns. Cached canonical states retain ordinary `Arc<Frame>` ownership. There is no global or persistent intern table, weak-entry cleanup, eviction or invalidation policy. Different reports and programs get independent tables. Already initialized canonical forms remain untouched, so repeated inspection and reporting do not reprocess them. Partially normalized records retain their existing continuation behavior.

Individual inspection, compact execution and exhaustive snapshot construction keep their existing paths. This change targets direct-path history materialization. Existing copy-on-write state operations still isolate mutations because a shared frame must be copied before modification. Neither source history nor compiled rules are modified.

The storage component lives in `canonical/storage.rs`; the snapshot renderer continues to own presentation text and node construction. The implementation uses standard ordered-set and shared-ownership types with no new dependencies or unsafe code. Temporary table storage is bounded by distinct frames encountered while initializing canonical forms in that report.

## Evidence and alternatives

A temporary diagnostic enumerated complete canonical object contents in the six-factor report. It found 136,303 frame entries with 8,763 distinct values: 21,348,520 bytes of frame bodies, shared-ownership headers and held-token capacity would require only 1,617,240 bytes if identical values shared storage. It also found approximately 27 MB of duplicate world data. These structural estimates exclude sequence containers and are recorded separately from allocator measurements. The diagnostic patch is archived and removed from production.

The first experiment reused an object only when renaming left it equal to its source. It saved less than 1 MB and slowed the preliminary export comparison, so it was removed. A temporary hash table sharing both worlds and frames saved approximately 47 MB but slowed that comparison by about 10%. Restricting the hash table to frames saved about 20 MB with a smaller latency cost. The accepted ordered set compares frame metadata before held-token contents and avoids hashing every full frame key. Preliminary comparisons motivated this choice; only the final alternating matrix establishes acceptance. The broader world-sharing experiment remains rejected in its measured form.

## Verification and measurement

The [raw audit](preservation.json) contains all preliminary and final samples, rejected implementation patches, the structural diagnostic, source/executable hashes, fixtures and reproduction scripts. Optimized Bazel executables run serially on the current Apple M5 Max on AC power, after builds and tests finish. Three rounds alternate baseline/candidate order. Each lifecycle process records a cold evaluation separately, followed by five fresh-instance samples. Compact execution and inspection use nine samples. Symmetry retains the existing 100 ms warm-up and 25 samples. Allocation diagnostics run separately from timing acceptance.

| Retained storage | Baseline | Candidate |
| --- | ---: | ---: |
| Six-factor engine | 150.09 MB | 130.36 MB |
| Six-factor owned report | 107.80 MB | 107.80 MB |
| Six-factor encoded buffer | 134.22 MB | 134.22 MB |
| Six-factor peak requested storage | 392.11 MB | 372.37 MB |
| Sixty-four-level engine | 2.405 MB | 0.578 MB |
| Sixty-four-level peak requested storage | 7.562 MB | 5.734 MB |

The six-factor engine retains approximately 19.7 MB less storage, a 13.1% reduction; whole-lifecycle peak falls 5.0%. The deep-scope fixture retains 76.0% less engine storage and has 24.2% lower peak storage. Every allocation diagnostic returns to zero additional tracked retained bytes after release. These counters measure requested Rust heap bytes, not allocator metadata, stack storage, process RSS or the WebAssembly heap.

| Workload | Baseline | Candidate |
| --- | ---: | ---: |
| Six-factor full reporting | 435.730 ms | 450.609 ms |
| Six-factor release | 16.937 ms | 14.673 ms |
| Six-factor complete lifecycle | 599.163 ms | 613.308 ms |
| Three-factor complete lifecycle | 117.329 ms | 119.895 ms |
| Nested complete lifecycle | 179.997 ms | 183.029 ms |
| Sixty-four-level complete lifecycle | 4.666 ms | 4.806 ms |
| Compact six-factor execution | 61.645 ms | 62.781 ms |
| Ten-digit decimal addition | 182.477 ms | 185.230 ms |
| Five-digit decimal multiplication | 207.109 ms | 208.578 ms |
| Incremental inspection | 5.384 ms | 5.596 ms |

All 27 protected comparisons pass the existing 5% regression tolerance, or 10% below one millisecond. The matrix includes eleven complete lifecycles, three compact arithmetic controls, incremental inspection and twelve symmetry cases. The full-export median is 2.4% slower: the retained-memory improvement justifies a measured latency tradeoff, not a speedup claim. The 512-distinct-world control gets no retained-memory benefit; the temporary table adds 104 requested bytes of allocation traffic and leaves its peak unchanged.

New ownership tests verify actual pointer sharing, preservation of canonical mappings, independent table lifetimes, copy-on-write isolation and release after the table and canonical owners are dropped. Capture tests keep structurally similar frames with different captured contexts distinct. A direct-path regression compares repeated reporting and resumed execution against canonical forms initialized without sharing, including tiny budgets, record-limit suspension, nested frames and executable-rule capture. Complete reports and runtime statistics agree at every tested boundary.

All 109 optimized Bazel test targets pass, including historical differential evaluation and native/WebAssembly conformance. The full suite executes 105 targets freshly and uses four cached results; the language target then passes again with the additional report/resume regression. Formatting and build-integrated checks pass. Sixteen complete and paused command reports compare byte-for-byte with the baseline, including a 69.6 MB complete report. No fields are filtered.

The remaining canonical world storage and complete report materialization still cost substantial memory. This change does not compress history, avoid canonicalization work, or establish a compact-execution speedup. An alternative world representation or scheduling improvement needs its own measurements and preservation argument.

```sh
bazel test -c opt //...
bazel run -c opt //:format -- --check
bazel run -c opt //benchmark:lifecycle -- expression '2*2*2*2*2*2' 2101 --sample 5
bazel run -c opt //benchmark:allocation -- expression '2*2*2*2*2*2' 2101 --sample 3
```
