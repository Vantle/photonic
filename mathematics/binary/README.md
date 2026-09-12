# Binary reference

The default arithmetic tool has moved to [ternary arithmetic](../arithmetic/README.md), implemented by the shared `mathematics/arithmetic` crate. This directory retains binary ordinary-rule examples and comparison data. There is no longer a `//mathematics/binary:word` executable; the new `//mathematics/arithmetic:word` command counts width in ternary digits.

## Sparse addition

```text
Add(2^0.2^1, 2^0.2^2)
[Add, Add] ()
[2^0.2^0] 2^1
[2^1.2^1] 2^2
[2^2.2^2] 2^3
```

This computes three plus five. Each power is an ordinary concept name; the caret is not an operator. Sparse carries preserve the weighted sum. The supplied `carry.particle` contains 32 carry rules and can produce position 32.

```sh
bazel run -c opt //system:command -- obsidian \
  "$PWD/mathematics/binary/addition.wave" \
  --target "$PWD/mathematics/binary/result.particle" \
  --steps 1000000 --states 2000 --cells 80
```

The known value in `product.particle` is a representation of 184500, not a multiplication program. Full-word binary circuits are generated and tested by the shared arithmetic implementation; duplicated generated archives have been removed.

## Comparisons and research

- [Current all-operation benchmark and commands](../arithmetic/README.md).
- [Binary/ternary sparse addition measurements](../ternary/README.md).
- [Representation comparison and historical balanced-layout measurement](representation.md).
- [Functional numeral investigation](../arithmetic/stream.md).

The historical benchmark reports were produced before the latest runtime index and shared-program optimization. Re-running these commands measures the current evaluator; old timings are not current guarantees.

```sh
bazel run -c opt //mathematics/binary:benchmark
bazel run -c opt //mathematics/arithmetic:benchmark
```
