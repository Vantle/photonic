# Ternary programs

The current [ternary arithmetic implementation](../arithmetic/README.md) supports all four word operations. Its runnable generated programs are in `add`, `subtract`, `multiply`, and `divide`; `digit.particle` exposes small digit rules. This page records the separate sparse carry construction and its historical radix comparison. Signed balanced-ternary arithmetic remains unimplemented.

The library uses ordinary concept names for powers of three. For example, six is `3^1.3^1`; nine is `3^2`. The caret is part of an atom name, not an exponent operator. The parser does not calculate these values.

```text
Add(3^1, 3^1.3^1)
[Add, Add] ()
[3^1.3^1.3^1] 3^2
```

This example adds three and six. A carry consumes three equal contributions and produces one contribution at the next position. Binary uses the same construction with two equal contributions. Both preserve their mathematical weighted sum, and neither supplies a native arithmetic operation.

The saved [addition program](addition.wave) proves 1500 + 123 = 1623:

```sh
bazel run -c opt //system:command -- obsidian \
  "$PWD/mathematics/ternary/addition.wave" \
  --target "$PWD/mathematics/ternary/result.particle" \
  --steps 1000000 --states 2000 --cells 80
```

There are 32 carry rules, from power zero through power 31. Unsupported higher positions are still ordinary concepts; the library does not automatically generate rules for them. This is sparse multiplicity: digit two occupies two occurrences, so fewer digit positions need not mean fewer total occurrences.

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
