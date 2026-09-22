# Shared report text

Baseline: `4a85dc9`. The [resource audit](resource.md) reduced canonicalization allocation calls without reducing peak retained storage. This change measures that storage and shares immutable presentation text within each report. It preserves program/rule execution and the complete serialized report.

## Ownership and representation

A private renderer owns a short-lived text table keyed by compiled symbol and a scope-name table keyed by scope index. It borrows one immutable compiled program. Both direct-path reports and exhaustive snapshots use one renderer across their nodes. Individual node inspection uses a fresh renderer, allocating entries only for the symbols and scopes actually inspected.

An atom's label and display share one `Arc<str>`. A rule's ordinal label and display name remain distinct strings with their original contents. Every token retains its own identifier and capture, and every frame retains its own parent and lexical references. Sharing presentation text does not merge tokens, frames or executable occurrences.

The renderer is dropped after construction. Nodes retain owned immutable string handles and remain valid after the renderer, compiled program and engine are destroyed. `Arc` preserves the report's ability to move between threads. Tables are private to one render operation, so program-local symbol indices cannot collide across programs and no cache invalidation or eviction policy is needed. Storage is bounded by the symbols and scopes encountered in that operation.

Snapshot data definitions remain in `snapshot.rs`; the new `render.rs` owns presentation construction and sharing. The previous node constructor is removed. Serde's existing `rc` feature enables ordinary serialization of the shared strings; no dependency version changes or new packages are involved. Native and WebAssembly builds retain the same Bazel dependency model.

## Rust interface

`Token.label`, `Token.display` and `Frame.scope` now hold `Arc<str>` instead of `String`. Rust callers can use `as_ref()` to borrow `&str` and `to_string()` when they need an independent mutable `String`. Repository callers have been updated. JSON field names, values, order and escaping remain identical. Event rule names and source program definitions retain their existing representation.

## Storage attribution

The allocation diagnostic now samples live requested bytes while dropping the engine, report and encoded buffer, in that order. It reports the net storage released by each owner group. The ordinary timing executable retains its original timer and release boundaries. The same attribution patch is present in the baseline diagnostic.

These counters measure requested Rust heap storage. They exclude allocator metadata, stack storage, process RSS and WebAssembly/JavaScript heap accounting. Sharing can make attribution depend on destruction order; the raw audit records the order, and the integration test checks that the three categories sum to the release phase's net change. They describe storage retained after serialization, which need not be the peak for every workload.

For the six-factor case, post-serialization attribution is:

| Owner group | Baseline | Candidate |
| --- | ---: | ---: |
| Engine | 168.8 MB | 168.8 MB |
| Owned report | 212.8 MB | 107.8 MB |
| Encoded JSON buffer | 134.2 MB | 134.2 MB |

Owned report storage falls by approximately 105 MB, or 49.3%. Peak live requested storage falls from approximately 515.8 to 410.8 MB, or 20.4%. The report still contains every state and event, and serialized output remains 125,450,547 bytes. Reporting allocation calls fall from 6,236,321 to 3,045,700. Every measured lifecycle returns to zero additional tracked retained bytes after release.

## Timing and verification

The [raw audit](render.json) records the baseline revision and instrumentation patch, source/executable hashes, fixtures, power state and all samples. Optimized Bazel-built executables run serially on the current Apple M5 Max on AC power, after builds and tests complete. Three rounds alternate baseline/candidate order. Lifecycle processes record their first evaluation separately and then five fresh-instance samples; compact execution and inspection use nine. Symmetry cases retain the same 100 ms warm-up and 25 samples on both sides. Allocation diagnostics are separate from timing acceptance.

| Workload | Baseline | Candidate | Baseline / candidate |
| --- | ---: | ---: | ---: |
| Six factors, full reporting | 522.601 ms | 464.125 ms | 1.13× |
| Six factors, release | 40.928 ms | 18.075 ms | 2.26× |
| Six factors, complete lifecycle | 718.467 ms | 639.436 ms | 1.12× |
| Three factors, complete lifecycle | 141.551 ms | 124.271 ms | 1.14× |
| Nested expression, complete lifecycle | 216.121 ms | 188.669 ms | 1.15× |
| Compact six-factor execution | 65.416 ms | 64.928 ms | 1.01× |
| Ten-digit decimal addition | 189.352 ms | 190.094 ms | 1.00× |
| Five-digit decimal multiplication | 213.583 ms | 212.393 ms | 1.01× |

All 26 protected comparisons pass the existing 5% regression tolerance, or 10% below one millisecond. Cases include ten complete lifecycles, three compact arithmetic controls, incremental inspection and twelve symmetry cases. Sharing benefits repeated text; a single node with 512 distinct atom names gets essentially no complete-program speedup and slightly more report storage. The temporary tables and shared-string headers therefore remain visible costs in the audit.

State/event/work counts, canonical search steps and report fingerprints agree across the paired runs. Sixteen complete or paused command reports also compare byte-for-byte, including a 69.6 MB complete report, paused products and captured-resource exploration. No fields are filtered out of those comparisons.

New tests check actual text sharing across report nodes and between atom labels/displays, scope-name sharing, report validity after engine destruction, transfer to another thread, and isolation between programs that reuse symbol indices. Both direct and exhaustive reporting paths are exercised. All 109 optimized Bazel test targets pass: 107 execute freshly and two unaffected targets are cached. The suite includes native/WebAssembly conformance and historical differential coverage. Formatting and build-integrated lint checks pass.

The remaining retained engine data and encoded output buffer account for most of this workload's peak after the change. Further reductions need separate attribution and a preservation argument; shared labels do not remove the cost of retaining canonical forms or full serialized history.

```sh
bazel test -c opt //...
bazel run -c opt //:format -- --check
bazel run -c opt //benchmark:lifecycle -- expression '2*2*2*2*2*2' 2101 --sample 5
bazel run -c opt //benchmark:allocation -- expression '2*2*2*2*2*2' 2101 --sample 3
```
