# Arithmetic and parallel execution

Status: research direction, with one implemented general evaluator optimization. Shared-prefix grouping is implemented as flat elaboration. A [retained-input two-numeral construction](natural/product.md) now has bounded execution evidence. The [binary library](binary/README.md) now implements compact carry addition and generated fixed-width multiplication; a generic direct-path strategy proves 1500 × 123. Complete-operand rule guards, arbitrary precision, and universal arithmetic certificates remain open. The older action representation remains a separate example.

## Correctness precedes the numeral representation

The current source-inference contract permits a later application to omit evidence-only outputs. Consequently `Tick.Unit.Unit [Tick.Unit] Tick.Result` reaches both one and two Results beside Tick. This is consistent with the existing reference case “Evidence-only co-results are not source leftovers.” It is not a regression introduced by arithmetic.

An experimental projection restriction required every source dependency consumed through inference to be exclusive to the selected witness outputs. It passed 67 of the then-current 68 Rust tests but failed that reference case, removing an accepted configuration. The experiment was reverted. No runtime semantic change is retained from it.

This counterexample does not prove that every general multiplication encoding is impossible. It does show that conventional accumulation loops cannot simply be assumed sound. Establish a fixed ordinary-rule program, with explicit input and result protocols, before optimizing or replacing its numeral representation. Do not add a counter-specific projection exception, implicit negative premise, or theorem-specific evaluator.

## Compact data and baseline algorithms

For positional numerals, retain the association between each digit and its position even when storage is orderless. Define decoding into the existing natural carrier, normalization, and equality; then prove each arithmetic transformation respects decoding. Position, bit, and limb notation must be implemented as ordinary data, not new native arithmetic syntax.

Start with carry-based binary addition and schoolbook multiplication as independently checkable baselines. Later compare recursive multiplication algorithms. GMP's manual documents size-dependent choices among basecase, Karatsuba, Toom variants, and FFT algorithms; one method is not optimal at every input size. [GMP multiplication algorithms](https://gmplib.org/manual/Multiplication-Algorithms).

The goal is useful arithmetic performance, not merely a faster unary demonstration. Unary output requires one occurrence per unit of numerical value. Parallel execution cannot remove that representation cost.

## Use parallel coherences for independent subproblems

Karatsuba provides a concrete future acceptance program. Write x = x₀ + Bx₁ and y = y₀ + By₁ for a positional base B. Compute p = x₀y₀, q = x₁y₁, and r = (x₀ + x₁)(y₀ + y₁) in independent coherences. Then combine p + B(r − p − q) + B²q. The middle difference is a natural because r includes p and q; a library can establish it using explicit addition evidence. Subtraction here is mathematical arithmetic, not a negative premise.

This is a known algorithm, not an invented Photonic algorithm. Its independent subproducts are a useful test of the language's parallel-world contract. Immutable operand representations may be shared, while each subcomputation introduces its own result resources. Joining shared input occurrences must not reinterpret them as independently produced output occurrences.

Expose parallel tasks only when their work exceeds coordination cost. Worker counts, task size, retained memory, arithmetic work, and proof-search work must all be measured separately. Existing tiny reference graphs are not evidence of scalable arithmetic speedup.

## Implemented: remove redundant symmetry orderings

The evaluator now identifies a conservative class of interchangeable coherences from resource incidence, frame identity, and captures. Canonicalization enumerates one ordering per certified class arrangement while retaining every coherence and every distinct resource. It uses the same semantics for atoms and rule values, with no arithmetic labels involved. See the [proof, benchmark, and limitations](../document/symmetry.md).

This is an application of graph symmetry reduction. Canonical labeling and graph automorphism algorithms are established work; nauty and Traces are a useful reference point for assessing more general techniques. [Nauty and Traces](https://users.cecs.anu.edu.au/~bdm/nauty/). The implementation here is a small sufficient certificate for swaps, not a replacement for those general algorithms or a novelty claim.

## Candidate research beyond that optimization

Investigate representing independent event combinations symbolically rather than enumerating every intermediate configuration. Resource consumption, reads, lexical captures, enabling interactions, and evidence support all contribute to dependence. Disjoint selected footprints alone do not certify independence.

Partial-order reduction is established research for reducing redundant interleavings. [Microsoft Research: transaction-based reduction](https://www.microsoft.com/en-us/research/publication/sound-transaction-based-reduction-without-cycle-detection/). Applying it here would require a Photonic-specific soundness argument. In particular Obsidian can query intermediate configurations: preserving only terminal results would change its contract. A symbolic representation would have to retain the omitted intermediate states' meaning and answer exact reachability correctly.

The possible contribution is a general execution algorithm that combines Photonic's provenance and coherence structure to avoid redundant work. There is no demonstrated new multiplication algorithm or superiority over existing arithmetic libraries yet.

## Acceptance sequence

1. Establish a correct fixed-program multiplier or explicitly propose the minimum general semantic extension it needs.
2. Resolve how complete-operand evidence participates in ordinary open matching. Shared-prefix syntax now elaborates to flat coherences; it does not introduce recursive containers or exact-match guards.
3. Extend the implemented compact addition and fixed-width multiplication with reusable conversion, arbitrary precision, and universally checked correspondence.
4. Compare independent coherence execution with one-worker execution using equivalent programs and equal correctness checks.
5. Compare known arithmetic algorithms across input sizes, including conversion and memory costs.
6. Attempt a new scheduling or arithmetic technique only with an explicit hypothesis, a correctness argument, and reproducible measurements.

These are unfinished milestones. The symmetry optimization is independently useful; it does not complete the arithmetic foundation.

The [representation comparison](binary/representation.md) now assesses higher radices, native limbs, redundant binary forms, and residue systems. The binary library also implements signed subtraction of unsigned operands and quotient/remainder division with an explicit zero-divisor result. These remain ordinary generated rules with no arithmetic runtime primitive.

The current default is the shared [ternary arithmetic implementation](arithmetic/README.md). It includes all four operations, domain-limited digit tables, dead-gate removal, immutable compiled-program sharing, and indexed matching. The recorded comparison favors column reduction over the balanced layout on the tested serial workload. This does not close the arbitrary-stream or universal-certificate milestones.
