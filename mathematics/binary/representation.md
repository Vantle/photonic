# Choosing a compact representation

Research and implementation assessment, 12 September 2026. Historical assessment, followed by implementation updates. The default has since moved to the [ternary word library](../arithmetic/README.md); binary remains a reference. Decimal conversion can remain an ordinary library concern. A balanced circuit alternative is implemented and measured below. Ternary sparse addition also has mixed measured results; no replacement has been established for all arithmetic operations.

## What the runtime pays for

A shorter numeral is not necessarily a cheaper program. The present circuit library pays for literal rule tables, fan-out particles, configuration canonicalization, and matching work at each event. The serial path strategy repeatedly invokes the same general runtime at a reachable successor. It does not exploit a hardware register, a native word multiply, or concurrent circuit evaluation.

The eleven-bit multiplier uses 1364 rules and 253 events. Its complete output has 22 positions. Unary 184500 would require 184500 occurrences just for the result. That representation reduction is established, but it says little about which compact encoding will minimize matching and evidence costs.

## Compare the candidates

| Representation | Potential benefit | Cost in this implementation | Recommendation |
| --- | --- | --- | --- |
| Complete binary | Small positive truth tables; explicit completeness | One atom per bit; carry and borrow dependencies | Keep as baseline |
| Base four or sixteen digit atoms | Fewer data occurrences and digit stages | More literal rule cases per position | Benchmark before adopting |
| Decimal digit atoms | Natural exact decimal input and output | Larger tables and radix conversion | Add when decimal use cases need it |
| Binary limbs | Compact native implementation of large integers | Native limb arithmetic would violate the current library-only contract; opaque atoms need decoding rules | Grouping alone is insufficient |
| Carry-save pairs | Postpone normalization; expose independent local reductions | Extra data, multiple encodings of one value, final normalization | Best next internal experiment |
| Signed digits | More choices for local arithmetic without long carries | More representations and normalization obligations | Candidate after carry-save |
| Residues modulo several bases | Independent component arithmetic | Range bounds, conversion, comparison, and general division | Useful specialized experiment, not a replacement |

The table's Photonic recommendations are engineering inferences, not benchmark results.

## Why larger digits do not automatically win

With literal full-subtractor tables, a radix-r digit has r choices for each operand and two incoming borrow values. That is `2r²` rules per position. For approximately w binary bits, there are `w / log₂(r)` positions, ignoring rounding. This simple construction therefore uses approximately `2r²w / log₂(r)` rules, excluding fan-out, selection, and output formatting.

| Radix | Cases per digit | Cases per represented binary bit |
| --- | ---: | ---: |
| 2 | 8 | 8 |
| 3 | 18 | about 11.4 |
| 4 | 32 | 16 |
| 10 | 200 | about 60.2 |
| 16 | 512 | 128 |

This is a derivation for explicit tables, not a lower bound for all possible encodings. Decomposing large digits into small gates avoids the table explosion but restores bit-level work and adds conversion. Smaller state size might still offset larger tables; only matched end-to-end benchmarks can decide.

GMP stores integers as sign and magnitude with arrays of binary limbs. Its single-limb division uses hardware division or multiplication by an inverse. Those mechanisms explain why packed words help a native arithmetic library, but their advantage cannot be assumed for an atom-only evaluator. [GMP integer representation](https://gmplib.org/manual/Integer-Internals), [single-limb division](https://gmplib.org/manual/Single-Limb-Division).

## Why carry-save is promising

Carry-save arithmetic retains a number as two contributions and uses independent full-adders to reduce three inputs to two; normalization eventually requires combining the retained contributions. This is an established representation, not a new algorithm. [Parhami, Computer Arithmetic, number representation, slides 47–52](https://web.ece.ucsb.edu/Faculty/Parhami/pres_folder/f31-book-arith-pres-pt1.pdf).

Our multiplier already uses local full-adders to compress product columns, then fully resolves each column. A balanced reduction graph that retains two rows until the final boundary could shorten dependencies. Chained arithmetic could also avoid repeatedly normalizing intermediate words. Both changes require an explicit library protocol and measurements; the current serial path strategy may gain little from reduced dependency depth alone.

For Photonic, numerical equivalence is not structural state identity. Two carry-save encodings of the same integer must not be silently merged by runtime canonicalization. A library can normalize them before an exact query, or explicitly prove their equivalence. This preserves the existing deduplication contract.

Independent reductions can be placed in coherences, but transferring operands and collecting results must preserve provenance and completeness. More worlds do not by themselves establish parallel speedup. First compare identical circuits with equivalent targets, then separate scheduler effects from arithmetic representation effects.

## Why residues are not the next default

Residue systems support componentwise modular arithmetic but general integer division remains difficult; a recent research paper develops dedicated decompositions for it. This is evidence of a tradeoff, not evidence of superiority for Photonic. [Eric B. Olsen, Direct Integer Division in RNS and its Hardware Solutions (2026 preprint)](https://arxiv.org/abs/2604.04796).

An exact residue library would also need an explicit range below the product of its moduli and a conversion/uniqueness argument. Without those, different integers can have the same residue tuple. Our immediate requirement includes ordinary subtraction, ordered remainders, and division, so canonical binary remains the simpler boundary.

## Historical balanced comparison

The circuit generator now supports both column and balanced reduction layouts, sharing all gate tables and emission logic. The balanced layout reduces triples across columns in rounds, then finishes with the same carry reducer. Both layouts pass arithmetic regressions.

For 1500 × 123 at eleven bits, a local optimized median of five samples measured:

| Layout | Source bytes | Events | Work steps | Time |
| --- | ---: | ---: | ---: | ---: |
| Column | 72460 | 253 | 1551422 | 837769 µs |
| Balanced | 72460 | 253 | 1533076 | 941132 µs |

The balanced version was about 12% slower in this serial sample despite slightly less work. The default remains column reduction. The then-saved column source was the baseline, and both variants prove the same exact target. Parsing, initialization, search, and report construction are timed; source construction and final destruction are excluded. Each layout ran five consecutive samples, so ordering and thermal effects are possible. This is one input and one evaluator, not a general comparison of hardware circuit depth or parallel speed.

```sh
bazel run -c opt //mathematics/arithmetic:benchmark
```

The [ternary comparison](../ternary/README.md) records four sparse-addition cases, including a counterexample to a universal ternary speedup. The [functional investigation](../arithmetic/stream.md) separates genuine reusable stream traversal from the still-missing generic arithmetic composition.

## Next experiment

Compare the current column reducer with a balanced carry-save reduction on the same operand widths, then benchmark base-four subtractor tables. Record source bytes, rules, live particles, events, work, retained evidence, conversion cost, and median wall time. Test the same exact numerical claims and incorrect targets. Keep the general runtime semantics fixed. Adopt another representation only if measured benefits survive its conversion and proof costs.
