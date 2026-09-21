# Residual certificates and causal history

This investigation follows the CPU continuation. Neither general algorithm below is enabled by this work. A restricted cached leaf certificate is implemented separately. Both can reduce repeated search, but preserving reachable answers alone is insufficient for the existing runtime contract.

## Residual assignment pruning

For `A.B.C,A,B.C [A,B,C] Done`, candidate domains are `A={0,1}`, `B={0,2}`, and `C={0,2}`. The whole query is feasible. After choosing `A=0`, the remaining two inputs compete for the single remaining world `2`. A residual distinct-world check can reject that branch without discovering a new binding. However, the current cursor still visits candidates, advances particle cursors, emits pending steps, and backtracks. Simply jumping over the branch changes the result of a bounded run.

The useful optimization boundary is an exact transcript certificate:

- The dependency key covers candidate occurrence identity, pattern equivalence, capture, input order, token alternatives and the starting cursor.
- The certificate proves that the skipped region emits no binding.
- It gives the exact logical waiting length and the exact continuation after that region.
- Execution consumes only the portion admitted by the caller's remaining budget. Interruption inside a certified span remains resumable.
- Invalid dependencies, unavailable certificates, arithmetic overflow, and record pressure fall back to scalar traversal.

Completed negative transcripts already meet much of this contract retrospectively. New completed-query replay extends reuse across compatible immutable candidate projections. First-visit certificates are harder: a cardinality failure alone does not give the number of token combinations and symmetry-rejected candidate visits that scalar traversal would perform. Computing that count can itself be expensive.

A worthwhile next experiment is a restricted exact counter for suffixes with one binding per candidate and a fixed input order. Exhaustively enumerate small domain matrices, compare every pending/binding step and continuation against the reference, and measure the counter including its dependency construction. Generalize only where the certificate remains cheaper than the traversal. This is a candidate experiment, not a claim that the general counting problem has been solved.

## Cached leaf certificate

A narrower certificate is available without counting an entire suffix. When an unchanged candidate already owns an immutable, nonviable particle preparation, revisiting it from an unscanned cursor position cannot emit a binding. Whether world compatibility rejects it or particle matching rejects it, the existing cursor advances to the next candidate and returns exactly one pending step. The optimized path performs that same transition directly. It does not prepare a previously unvisited candidate, skip any logical step, or reject an entire residual subtree. Occurrence replacement removes the corresponding member and its preparation.

The direct cursor also maintains a machine-word membership mask for occupied sites below 64; larger site identifiers use the original exact scan. This is a representation fast path for resource distinctness, independent of program syntax and arithmetic. Push, pop, reset and seeded-prefix restoration maintain the mask. Debug tests compare each evaluated compatibility decision against the original full scan, and an independent 70-world test crosses the mask boundary.

## Causal-history compression

`A,C [A] B [C] D` admits two orders of independent firings. A reachability-only engine could choose one order when checking a terminal value. Photonic also exposes intermediate states, events, source-specific flow and support. Removing an order can remove observable structure even when both orders reach equivalent terminal values.

Independence must cover more than disjoint consumed resources. Executable-rule reads, lexical frames, captured resources, fresh identity allocation, normalization, and downstream evidence all matter. Equal output labels do not imply equal derivations; unsupported cycles and incompatible histories must remain distinguishable.

The compatible direction is representation compression: retain a partial-order description and lazily reconstruct each currently observable history in its original reporting and budget order. This might reduce retained storage and repeated internal construction. It cannot eliminate the cost of actually enumerating all histories when the caller requests them. Any change that instead reports a quotient of histories requires a separately approved semantic contract.

Before implementing such compression:

1. Specify a dependency-based independence relation and prove the two local firing orders preserve state, source flow, captures and grounded support under an explicit identity mapping.
2. Define reconstruction of intermediate states, event identities and support clauses, including suspended runs.
3. Test independent firings, shared reads, consumed executable code, captured frames, competing production and unsupported cycles against full reference snapshots.
4. Measure representation construction, reconstruction, peak storage and complete reporting. Reject a compressed representation that merely moves an unacceptable cost to inspection.

The historical differential harness now includes 128 successive one-work-unit observations for each example above. This guards their present bounded behavior; it is not a proof of either proposed optimization.

## Research connection

[Free Join](https://arxiv.org/abs/2301.10841) motivates workload-sensitive combinations of join strategies. [Shared Arrangements](https://www.vldb.org/pvldb/vol13/p1793-mcsherry.pdf) supports separating reusable indexed data from independent consumers. [Unfolding-based Partial Order Reduction](https://arxiv.org/abs/1507.00980) supplies a foundation for exploring independent executions through partial orders. Applying these ideas while reconstructing Photonic's exact observations is an additional engineering and equivalence obligation.
