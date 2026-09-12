# Finite decimals

Finite nonnegative decimals can be specified without changing Photonic. This is a mathematical representation proposal with one executable common-scale addition protocol. Arbitrary scale alignment, normalization, decimal parsing, and universal certificates are not implemented.

## Representation

Use a pair of naturals (n, k), with mathematical value n / 10^k. Store n as Unit occurrences in one Coefficient coherence and k as Place occurrences in one Scale coherence. Both labels and Place are ordinary concepts; the two components are explicitly tagged and remain present even when empty.

For example, this represents 0.3:

```text
Coefficient.Unit.Unit.Unit,
Scale.Place
```

Zero at scale zero is `Coefficient, Scale`. These are two labeled coherences, distinct from the one empty coherence representing natural zero. This first interface carries one decimal pair; a collection of pairs needs an explicit association protocol so coefficients and scales cannot be accidentally crossed.

Decimal punctuation and the pair notation in this document are explanatory mathematics, not new Photonic syntax. The runtime treats `0.3` as concepts separated by a dot, not as a decimal literal.

## Numerical equality

Identify (n, k) and (m, l) when n × 10^l = m × 10^k. These operations initially refer to ordinary mathematical natural arithmetic, whose universally checked Photonic proof library is not yet implemented. Reflexivity and symmetry follow from natural equality. For transitivity, multiply the two equalities by the missing powers and cancel the common positive power of ten. Thus this is an equivalence relation.

In particular (n, k) and (10n, k + 1) represent the same value. Zero at every scale represents the same value. Runtime structural equality does not implement this quotient: Obsidian distinguishes their different coefficient and scale occurrences. Decimal numerical equality will require an explicit library computation and evidence protocol.

A canonical presentation has k = 0 or a coefficient not divisible by ten. Repeated exact division by ten while decreasing positive k terminates because k decreases. If two canonical presentations had different scales, the coefficient at the larger scale would be divisible by ten, a contradiction. At equal scales cross multiplication and cancellation make their coefficients equal. Zero consequently normalizes to (0, 0). This is a mathematical algorithm specification, not an implemented guard or negative premise; a future program must produce explicit quotient/remainder evidence through positive rules.

## Addition

At a common scale k, define (n, k) + (m, k) = (n + m, k). For different scales choose K = max(k, l), align coefficients to n × 10^(K-k) and m × 10^(K-l), then add at K. Raising the chosen common scale multiplies both coefficients by the same power of ten and preserves the represented value. Equivalent input presentations therefore produce equivalent sums. Identity, associativity, commutativity, and cancellation follow from the corresponding natural laws after aligning to one common scale.

The [runnable example](addition.wave) adds 0.3 and 0.7 at an explicitly shared scale of one:

```text
Add.Unit.Unit.Unit,
Add.Unit.Unit.Unit.Unit.Unit.Unit.Unit,
Scale.Place
[Add, Add] Coefficient
```

The two Add coherences hold independently introduced coefficients. The rule rejoins them as Coefficient; the unrelated Scale coherence stays intact. The [exact target](result.particle) has ten Units and one Place: 1.0. This program accepts coefficients already at a common scale. It neither verifies two independently supplied scales nor normalizes 1.0 to 1.

```sh
bazel run -c opt //system:command -- obsidian "$PWD/mathematics/decimal/addition.wave" --target "$PWD/mathematics/decimal/result.particle" --cells 32
```

The explicit cell budget accommodates both coefficient occurrences and scale metadata; the default budget of twelve is too small for this example.

The scale must not be broadcast into both operands and accidentally counted twice; keeping one separate Scale coherence makes its ownership explicit. No special metaprogramming is necessary for this concrete operation. Reusable scaling programs remain a future ordinary-rule library task.

## Boundary

This carrier covers exactly nonnegative fractions with a power-of-ten denominator, not all rationals or reals. Addition and multiplication stay inside it: multiplication uses (nm, k + l). Division is not closed; 1/3 has no finite decimal expansion. Negative values require a signed-number construction. Infinite expansions and real-number completeness require another representation.

Unary coefficients and scales are deliberately transparent but grow quickly. Before building practical decimal arithmetic, extend the [multiplication action protocol](../natural/multiplication.md) to decimal coefficient inputs and implement division with remainder, then implement and verify alignment and normalization. A positional digit encoding could improve efficiency without native number syntax, but its correspondence must be established separately.

## Place value in an unordered representation

Orderless particles preserve multiplicity, but do not infer digit positions. A compact positional representation must explicitly associate each digit with its exponent; a flat pile of Digit and Power labels would lose those associations. The current pair avoids this issue by tagging a single coefficient and its single global scale.

A proposed notation such as `*(3.^(10))` should remain explanatory until an ordinary-rule encoding is demonstrated. It is not a new operator or grammar proposal here. Whole-rule values may provide inert association data under a fixed protocol, but extracting and evaluating arbitrary digit/exponent pairs is not assumed to work automatically. Develop that representation after numeral-to-action conversion and scaling have executable definitions.
