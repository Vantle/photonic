# Runtime review

This increment reviews the complete runtime for repeated work, allocation and duplicated structure, then removes what the review found without changing any observable result. Its baseline is the checkpoint commit `6923c5d`, which contains the population, admission, transcript, incidence, dependency and identity increments. The [raw audit](review.json) contains the paired measurements, allocation counts, phase profiles, the commit list and reproduction commands.

## Changes

**Hashing.** Every runtime `HashMap`, `HashSet` and `IndexSet` used the randomized SipHash builder, although their keys are interned integers and their iteration order is never observable. Sampling attributed about 12% of six-factor execution to SipHash alone. `hashing::Builder` supplies a deterministic multiplicative hasher in the style of rustc's. State fingerprints keep their original `DefaultHasher` definition, so no canonical ordering input changes.

**Dispatch.** Enabled, altered and empty-pattern inputs are dense catalog indices. They now live in `mask::Set`, a word mask, instead of a sorted set that became a B-tree beyond 32 members. Admission no longer resets joins that dispatch always resets before their first step. Consumer replacement adjusts storage by the consumer count, request priorities are recorded when requests are built instead of recomputed on every comparison, the ready set changes only when viability flips, and the sorted removal list is searched logarithmically. The network keeps one selection mask and one request buffer across events and refills them in place instead of allocating both for every affected frame.

**Reachability.** About 30% of arithmetic transitions recompute frame reachability, and each recomputation walked every context member, including the 837 root rules of the arithmetic program. A population now records the capture shared by all of its backing tokens. Frame references yield that capture once and scan members only when captures are mixed. The reachability index seeds its closure from its anchor counts instead of rescanning coherences. Reachable sets and network edge sets are unchanged for every state, including hand-built states whose context members capture foreign frames.

**Index and admission.** A vocabulary counts live context symbols, so a visibility probe walks the lexical ancestry only when some context holds the symbol. Admission first confirms that every term has a posting or a visible occurrence, which rejects most of the roughly 66,000 failing admissions in the six-factor run before any domain is allocated. Symbol presence and context membership bookkeeping, previously repeated five times, now lives in `acquire`, `release`, `enter` and `leave`. Each update rebuilt `index.affected` as a hash map of per-frame hash sets. `affected::Set` now keeps one sorted frame list and one sorted list of frame and symbol pairs, cleared and reused across updates. A frame affected only by an empty coherence stays affected with no symbols, and record accounting is unchanged.

**Transition and binding.** Output worlds collect the borrowed remainder once instead of cloning it at exact capacity and then reallocating. The first vacant frame is found lazily. Consumption groups the already-sorted context places instead of rebuilding them in B-trees. Binding selection detects repeated places by length. Sets of up to eight elements and lists of up to 256 values are collected with one allocation. A fingerprint world with a single dependency keeps it inline, fingerprint buckets hold their usual single state inline, and the reachability closure keeps its scratch space on the stack for up to 64 frames.

**Compilation.** Rules are interned by a `Form` of already-interned symbols: sorted input and output particles and sorted body rule indices. This is exactly the canonical definition identity, because each child symbol denotes its canonical class, and it avoids an allocating canonical copy of every definition. Numbering, names and scopes are unchanged. `Program::new` and `Runtime::new` borrow their source instead of forcing each search to clone it. Target compilation resolves rules and atoms by lookup and copies the program only for a target that mentions a symbol the program never interned, so it still never alters the execution program.

**Exhaustive runtime.** Support initialization borrows its clauses instead of cloning each one, and a premise is a sorted inline vector instead of a B-tree. Flow composition preserves the event relation's sorted keys instead of rebuilding and re-sorting it. Delivery reads reuse the normalized read set, and dense request activity flags live in a vector instead of a hash set.

**Structure and interfaces.** The direct search performs its normalization steps through one `normalize` operation with unchanged work accounting. The outcome expression, pending-limit admission, runtime snapshot assembly and the exhaustive cursor's next-candidate scan each exist once. `prism::Search::snapshot` serves callers that need only the execution, without cloning the source program and target. Command JSON is written through a 64 KiB buffer; the locked standard output is line-buffered, so pretty reports issued one write per output line. The browser parses the formula program once instead of on every query, and its product demonstration folds each column's incoming carry onto the subtotal, cutting digit searches per product from 44 to 28.

## Validation

All 110 optimized Bazel test targets pass, including 289 runtime tests, the frontend, command and program suites, WebAssembly conformance and the exact expression record. The release headless Chrome check of the webbook and its WebAssembly sandbox passes. Formatting and Clippy pass. Every paired benchmark observation agrees: work, event and state counts, lifecycle report fingerprints and byte counts, streaming export fingerprints, exhaustive record and peak counts, and symmetry step counts and completion.

New tests cover the word mask and the affected set against B-tree oracles, including frames with no symbols, the population capture summary through consumption, clearing and test mutation, and symbol-form interning: order-insensitive identity across particles, inputs, outputs, nested rule values and bodies; distinct multiplicities and outputs; pre-order numbering with source and fallback names; and target resolution with and without unknown symbols, leaving the execution program unchanged. The bounded transition oracle now also checks production reachability against its independent scan for every explored state.

## Measurements

Optimized native binaries of the checkpoint and of `71dc85e` ran sequentially on the development Apple M5 Max. Other work could share the machine, so each comparison alternates the two binaries in paired rounds: five rounds for compact workloads, three for streaming export, four rounds of 2,001 samples for small lifecycles, and three for the exhaustive suite, inspection and symmetry controls. Values are medians of round medians.

| Workload | Before | After | Change |
| --- | ---: | ---: | ---: |
| `2+2` execution | 9.097 ms | 5.356 ms | 1.70× faster |
| `2*2*2*2*2*2` execution | 76.241 ms | 43.936 ms | 1.74× faster |
| `(2+2)*(2+2)` execution | 30.671 ms | 17.982 ms | 1.71× faster |
| Arithmetic search initialization | 4.782 ms | 2.042 ms | 2.34× faster |
| 64 ordinary transitions | 0.127 ms | 0.091 ms | 1.40× faster |
| 32 body entries and returns | 0.179 ms | 0.136 ms | 1.32× faster |
| 32 context consumptions | 0.191 ms | 0.114 ms | 1.67× faster |
| 256 staged context consumptions | 1.528 ms | 1.202 ms | 1.27× faster |
| Availability, 64 wide and 64 deep | 1.996 ms | 1.338 ms | 1.49× faster |
| Availability, 256 wide and 16 deep | 8.493 ms | 5.884 ms | 1.44× faster |
| Availability, 4,096 wide | 95.646 ms | 78.725 ms | 1.21× faster |
| Repeated inspection, every step | 14.948 ms | 7.986 ms | 1.87× faster |
| Repeated inspection, every 64 steps | 12.990 ms | 7.317 ms | 1.78× faster |
| Six-factor streaming export | 2.436 s | 2.057 s | 1.18× faster |

All 19 exhaustive reference cases take less time, by 15.2–29.1% with a median of 21.1%. The seven symmetry controls above 3 µs change by between 7.9% faster and 1.1% slower, with a median of 2.8% faster. The five controls near 2 µs differ by at most 5.6%, one or two ticks of the 42 ns timer.

In the allocation-instrumented build, six-factor initialization requests 117,035 allocations instead of 215,563, and execution 534,539 instead of 744,581, with requested execution bytes falling from 107.65 to 76.29 MB. Reporting requests 0.9% fewer allocations for the same bytes, and retained engine storage falls from 439.5 to 438.2 MB. Requested bytes are not RSS or WebAssembly heap measurements.

## Rejected trials

Each rejected variant was measured against its predecessor during development and removed:

- **Unstable renaming sorts.** Switching the two renaming sorts to unstable sorts made streaming export 8% slower. Resources arrive in long presorted runs, which the stable merge sort handles nearly linearly.
- **Dense render caches.** Index-addressed render text caches made export 1.6% slower across six rounds.
- **Binary-searched consumption.** Removing context members by binary search over each consumed group made 256 staged consumptions 6.6% slower. A direct comparison against the group's few places is kept instead.
- **Stack-first set collection.** Collecting every `Set` into an eight-element stack buffer, including singletons, made the small scope lifecycle 3% slower. Singletons keep their early exit.
- **Whole-list buffering.** Buffering entire lists above 256 values before building the persistent tree made the 4,096-wide availability control 2.4% slower. Only the first 256 values are buffered.

## Remaining work

The phase-instrumented six-factor profile of `71dc85e` attributes execution as follows:

- **Dispatch, about 42%.** Subscription maintenance takes 29% of execution: it builds request lists for each affected frame, removes stale subscriptions and admits new ones. Admission alone takes 14%.
- **Index maintenance, about 19%.** Postings, readers and context membership change with every event.
- **Transition, about 10%, and fingerprint maintenance, about 10%.** The checkpoint already shares the fingerprint dependency index between events.
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
bazel run -c opt //benchmark:allocation -- expression '2*2*2*2*2*2' 2101 --sample 1
bazel run -c opt //benchmark:profile -- '2*2*2*2*2*2' 2101 --sample 3
```
