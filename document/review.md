# Runtime review

This increment reviews the complete runtime for repeated work, allocation and duplicated structure, then removes what the review found without changing any observable result. Its baseline is the checkpoint commit `6923c5d`, which contains the population, admission, transcript, incidence, dependency and identity increments. The [raw audit](review.json) contains the paired measurements, the commit list and reproduction commands.

## Changes

**Hashing.** Every runtime `HashMap`, `HashSet` and `IndexSet` used the randomized SipHash builder, although their keys are interned integers and their iteration order is never observable. Sampling attributed about 12% of six-factor execution to SipHash alone. `hashing::Builder` supplies a deterministic multiplicative hasher in the style of rustc's. State fingerprints keep their original `DefaultHasher` definition, so no canonical ordering input changes.

**Dispatch.** Enabled, altered and empty-pattern inputs are dense catalog indices. They now live in `mask::Set`, a word mask, instead of a sorted set that became a B-tree beyond 32 members. Admission no longer resets joins that dispatch always resets before their first step. Consumer replacement adjusts storage by the consumer count, request priorities are recorded when requests are built instead of recomputed on every comparison, the ready set changes only when viability flips, and the sorted removal list is searched logarithmically.

**Reachability.** About 30% of arithmetic transitions recompute frame reachability, and each recomputation walked every context member, including the 837 root rules of the arithmetic program. A population now records the capture shared by all of its backing tokens. Frame references yield that capture once and scan members only when captures are mixed. The reachability index seeds its closure from its anchor counts instead of rescanning coherences. Reachable sets and network edge sets are unchanged for every state, including hand-built states whose context members capture foreign frames.

**Index and admission.** A vocabulary counts live context symbols, so a visibility probe walks the lexical ancestry only when some context holds the symbol. Admission first confirms that every term has a posting or a visible occurrence, which rejects most of the roughly 66,000 failing admissions in the six-factor run before any domain is allocated. Symbol presence and context membership bookkeeping, previously repeated five times, now lives in `acquire`, `release`, `enter` and `leave`.

**Transition and binding.** Output worlds collect the borrowed remainder once instead of cloning it at exact capacity and then reallocating. The first vacant frame is found lazily. Consumption groups the already-sorted context places instead of rebuilding them in B-trees. Binding selection detects repeated places by length. Sets of up to eight elements and lists of up to 256 values are collected with one allocation.

**Compilation.** Rules are interned by a `Form` of already-interned symbols: sorted input and output particles and sorted body rule indices. This is exactly the canonical definition identity, because each child symbol denotes its canonical class, and it avoids an allocating canonical copy of every definition. Numbering, names and scopes are unchanged. `Program::new` and `Runtime::new` borrow their source instead of forcing each search to clone it. Target compilation resolves rules and atoms by lookup and copies the program only for a target that mentions a symbol the program never interned, so it still never alters the execution program.

**Exhaustive runtime.** Support initialization borrows its clauses instead of cloning each one, and a premise is a sorted inline vector instead of a B-tree. Flow composition preserves the event relation's sorted keys instead of rebuilding and re-sorting it. Delivery reads reuse the normalized read set, and dense request activity flags live in a vector instead of a hash set.

**Structure and interfaces.** The direct search performs its normalization steps through one `normalize` operation with unchanged work accounting. The outcome expression, pending-limit admission, runtime snapshot assembly and the exhaustive cursor's next-candidate scan each exist once. `prism::Search::snapshot` serves callers that need only the execution, without cloning the source program and target. Command JSON is written through a 64 KiB buffer; the locked standard output is line-buffered, so pretty reports issued one write per output line. The browser parses the formula program once instead of on every query, and its product demonstration folds each column's incoming carry onto the subtotal, cutting digit searches per product from 44 to 28.

## Validation

All 110 optimized Bazel test targets pass, including 288 runtime tests, the frontend, command and program suites, WebAssembly conformance and the exact expression record. The release headless Chrome check of the webbook and its WebAssembly sandbox passes. Formatting and Clippy pass. Every paired benchmark observation agrees: work, event and state counts, lifecycle report fingerprints and byte counts, streaming export fingerprints, exhaustive record and peak counts, and symmetry step counts and completion.

New tests cover the word mask against a B-tree oracle, the population capture summary through consumption, clearing and test mutation, and symbol-form interning: order-insensitive identity across particles, inputs, outputs, nested rule values and bodies; distinct multiplicities and outputs; pre-order numbering with source and fallback names; and target resolution with and without unknown symbols, leaving the execution program unchanged. The bounded transition oracle now also checks production reachability against its independent scan for every explored state.

## Measurements

Optimized native binaries ran sequentially on the development Apple M5 Max. Another agent was active on the same machine, so each comparison alternates the checkpoint and review binaries in paired rounds: five rounds for compact workloads, three for streaming export, four rounds of 2,001 samples for small lifecycles, and three for the exhaustive suite, inspection and symmetry controls. Values are medians of round medians.

| Workload | Before | After | Change |
| --- | ---: | ---: | ---: |
| `2+2` execution | 9.248 ms | 5.707 ms | 1.62× faster |
| `2*2*2*2*2*2` execution | 77.520 ms | 47.069 ms | 1.65× faster |
| `(2+2)*(2+2)` execution | 31.159 ms | 19.228 ms | 1.62× faster |
| Arithmetic search initialization | 4.879 ms | 2.106 ms | 2.32× faster |
| 64 ordinary transitions | 0.129 ms | 0.101 ms | 1.27× faster |
| 32 body entries and returns | 0.180 ms | 0.150 ms | 1.20× faster |
| 32 context consumptions | 0.194 ms | 0.120 ms | 1.61× faster |
| 256 staged context consumptions | 1.524 ms | 1.252 ms | 1.22× faster |
| Availability, 64 wide and 64 deep | 2.010 ms | 1.391 ms | 1.44× faster |
| Availability, 256 wide and 16 deep | 8.609 ms | 5.892 ms | 1.46× faster |
| Availability, 4,096 wide | 97.190 ms | 79.239 ms | 1.23× faster |
| Repeated inspection, every step | 15.037 ms | 8.395 ms | 1.79× faster |
| Repeated inspection, every 64 steps | 13.167 ms | 7.644 ms | 1.72× faster |
| Six-factor streaming export | 2.445 s | 2.091 s | 1.17× faster |

All 19 exhaustive reference cases are faster, by 12.8–28.4% with a median of 20.7%. The seven symmetry controls above 3 µs change by between 7.0% faster and 2.7% slower, with a median of 1.7% faster; the five controls near 2 µs vary within ±5% between runs, which is close to timer resolution.

In the allocation-instrumented build, six-factor initialization requests 117,049 allocations instead of 215,563, and execution 612,566 instead of 744,581, with requested execution bytes falling from 107.65 to 93.39 MB. Reporting allocation and retained engine storage are unchanged. Requested bytes are not RSS or WebAssembly heap measurements.

## Rejected trials

Each rejected variant was measured against its predecessor during development and removed:

- **Unstable renaming sorts.** Switching the two renaming sorts to unstable sorts made streaming export 8% slower. Resources arrive in long presorted runs, which the stable merge sort handles nearly linearly.
- **Dense render caches.** Index-addressed render text caches made export 1.6% slower across six rounds.
- **Binary-searched consumption.** Removing context members by binary search over each consumed group made 256 staged consumptions 6.6% slower. A direct comparison against the group's few places is kept instead.
- **Stack-first set collection.** Collecting every `Set` into an eight-element stack buffer, including singletons, made the small scope lifecycle 3% slower. Singletons keep their early exit.
- **Whole-list buffering.** Buffering entire lists above 256 values before building the persistent tree made the 4,096-wide availability control 2.4% slower. Only the first 256 values are buffered.

## Remaining work

A sampled six-factor profile on the review branch attributes execution as follows:

- **Subscription maintenance, about 37%.** Dispatch reconstructs request lists for each affected frame and drops and readmits subscriptions.
- **Index maintenance, about 22%.** `index.affected` recreates its per-frame symbol sets on every event. A flat sorted representation must still distinguish frames affected by an empty coherence.
- **Transition, about 12%, and fingerprint maintenance, about 10%.** The checkpoint already shares the fingerprint dependency index between events.
- **Matching, about 6%.** This is the only share spent finding bindings.

Complete reports still spend most of streaming export canonicalizing each state from scratch. Candidates include memoizing rendered frames shared between nodes and canonicalizing report states in parallel ahead of serialization.

```sh
bazel test -c opt //...
bazel test --config=release //toolchain/browser:check
bazel run -c opt //:format -- --check
bazel run -c opt //benchmark:expression -- '2*2*2*2*2*2' 2101 --sample 15
bazel run -c opt //benchmark:lifecycle -- expression '2*2*2*2*2*2' 2101 --sample 3 --export view --writer
bazel run -c opt //benchmark:runtime
bazel run -c opt //benchmark:inspection -- --length 32 --interval 1 --sample 21
bazel run -c opt //benchmark:symmetry
```
