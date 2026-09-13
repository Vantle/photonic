# Ternary arithmetic

Ternary uses digits zero, one, and two. The [arithmetic implementation](../arithmetic/README.md) supplies addition, subtraction, multiplication, and division through ordinary Photonic rules.

## Write 47 directly

47 is 1202 in base three: 1 × 27 + 2 × 9 + 0 × 3 + 2 × 1.

For the sparse carry library, write [numeral.particle](numeral.particle):

```text
3^3.3^2.3^2.3^0.3^0
```

Each occurrence contributes one power of three. The repeated powers encode coefficient two; no occurrence is needed for the zero coefficient. Ordering does not matter. These are ordinary concept names: the caret is part of each name, not an exponent operator. The carry library supplies their computational relationships.

For a complete positional word, write [word.particle](word.particle):

```text
Digit3One.Digit2Two.Digit1Zero.Digit0Two
```

Each atom states both position and value. Position zero is the units column. Zero digits remain explicit in this representation. These labels describe a word; a generated circuit consumes its wired Input facts. The sparse carry and positional circuit interfaces are separate encodings.

A bare `1202` or `47` is an ordinary atom, not a built-in numerical literal. If a particular program wants a short named numeral, it can define one explicitly:

```text
47
[47] 3^3.3^2.3^2.3^0.3^0
```

That rule gives this one name a meaning. It does not add a decimal parser or automatically interpret other atom names.

## A handwritten addition

This complete source adds one to the sparse numeral 47:

```text
Add.3^3.3^2.3^2.3^0.3^0,
Add.3^0
[Add, Add] ()
[3^0.3^0.3^0] 3^1
```

The exact result is `3^3.3^2.3^2.3^1`, representing 48. The regression uses the shared carry library and checks both that result and rejection of the unchanged 47 target after closed exploration.

## Files by purpose

| File | Purpose |
| --- | --- |
| [carry.particle](carry.particle) | Sparse power carry rules, positions zero through 31 |
| [digit.particle](digit.particle) | Standalone local digit operations |
| [numeral.particle](numeral.particle) | Handwritten sparse numeral 47 |
| [word.particle](word.particle) | Handwritten complete positional numeral 47 |
| [sum.wave](sum.wave), [sum.particle](sum.particle) | Sparse 1500 + 123 example and target |
| [add.wave](add.wave), [add.particle](add.particle) | Positional addition and target |
| [subtract.wave](subtract.wave), [subtract.particle](subtract.particle) | Positional signed subtraction and target |
| [multiply.wave](multiply.wave), [multiply.particle](multiply.particle) | Positional multiplication and target |
| [divide.wave](divide.wave), [divide.particle](divide.particle) | Positional quotient/remainder division and target |
| [successor.particle](successor.particle) | Reusable stream digit transformation rules |
| [stream.wave](stream.wave), [done.particle](done.particle) | Input stream and completion target |

`.wave` denotes runnable source by convention; `.particle` denotes reusable rules or data. They have identical grammar. Operations and targets sit together in this directory, without per-operation folders.

`Digit` names output digits without repeating the radix in every atom. `Carry` and `Borrow` name local arithmetic information; `Input` and `Port` identify generated wiring. `Quotient`, `Remainder`, `Negative`, and `Undefined` identify result roles. These are library conventions, not reserved language words.

The sparse representation uses multiplicity: a coefficient of two occupies two occurrences. A complete positional word uses one occurrence per digit, including zeros. Choose the representation expected by the program you are running.

## Measured comparison

Historical local optimized execution before the latest runtime index, median of three samples per case. Every search reached its exact target with closed exploration. Timing includes parsing, initialization, full search, and report construction; external source construction and final destruction are excluded. These examples deliberately include carry chains in both bases; they are not a representative workload distribution.

| Addition | Binary states | Ternary states | Binary time | Ternary time |
| --- | ---: | ---: | ---: | ---: |
| 1500 + 123 | 48 | 8 | 12438 µs | 901 µs |
| 184500 + 123 | 16 | 4 | 1628 µs | 225 µs |
| 59048 + 1 | 2 | 22 | 146 µs | 2998 µs |
| 65535 + 1 | 34 | 2 | 6475 µs | 142 µs |

Ternary won three cases; binary was about twenty times faster on the ternary carry chain. This supports making radix a library choice, rather than claiming one base universally minimizes Photonic states. These results measure sparse addition, not fixed-width circuit multiplication or division.

```sh
bazel run -c opt //mathematics/binary:benchmark
```

Small cases in both bases are checked with closed exploration, including preservation of the weighted sum in every reached state. The source generator for this comparison is shared; only the radix changes.

## Balanced ternary

Balanced ternary uses digits −1, 0, and +1 with base three. It is distinct from the unsigned 0/1/2 multiplicity representation above. [Parhami, Computer Arithmetic, slide 15](https://web.ece.ucsb.edu/Faculty/Parhami/pres_folder/f31-book-arith-pres-pt1.pdf).

Such digits could be ordinary positive data values in Photonic; a negative digit is not a negative premise. A candidate library would need explicit normalization, carry, comparison, and division rules. It remains an experiment to design and measure, not a current language mode.
