# Ternary arithmetic

The default arithmetic tool now uses base three for addition, signed subtraction of natural operands, multiplication, and quotient/remainder division. One shared circuit implementation supports ternary and the retained binary benchmark. All execution uses ordinary Photonic rules and the existing runtime.

This is the best measured construction we currently have for the demonstrated workload. It is not a claim of globally optimal arithmetic, arbitrary precision, or a completed functional numeral calculus.

## Run all four operations

From the repository root:

```sh
bazel run -c opt //mathematics/arithmetic:word -- \
  --operation add --left 1500 --right 123 --expected 1623

bazel run -c opt //mathematics/arithmetic:word -- \
  --operation subtract --left 1500 --right 123 --expected 1377

bazel run -c opt //mathematics/arithmetic:word -- \
  --operation multiply --left 1500 --right 123 --expected 184500

bazel run -c opt //mathematics/arithmetic:word -- \
  --operation divide --left 1500 --right 123 --expected 12 --remainder 24
```

The expected result is a proposed exact target. The external tool encodes it; Photonic proves reachability. The tool does not calculate the answer from the input operands. Unknown means this proof path failed or hit a limit, not that the mathematical claim is false.

| Operation | Decimal result | Ternary result, most significant digit first |
| --- | --- | --- |
| Add | 1623 | 02020010 |
| Subtract | 1377 | 1220000, nonnegative |
| Multiply | 184500 | 00100101002100 |
| Divide | quotient 12, remainder 24 | quotient 0000110, remainder 0000220 |

Leading zeros reflect the complete output width. These displayed digit strings are explanatory notation; they are not new Photonic numerical literals.

## The actual programs

| Operation | Ordinary Photonic source | Exact target |
| --- | --- | --- |
| Add | [program](../ternary/add/program.wave) | [target](../ternary/add/target.particle) |
| Subtract | [program](../ternary/subtract/program.wave) | [target](../ternary/subtract/target.particle) |
| Multiply | [program](../ternary/multiply/program.wave) | [target](../ternary/multiply/target.particle) |
| Divide | [program](../ternary/divide/program.wave) | [target](../ternary/divide/target.particle) |

For example, execute the saved multiplication program directly, without the generator:

```sh
bazel run -c opt //system:command -- obsidian \
  "$PWD/mathematics/ternary/multiply/program.wave" \
  --target "$PWD/mathematics/ternary/multiply/target.particle" \
  --path --steps 20000000 --states 4096 --records 1000000 --cells 4096
```

Add `--directory "$PWD/mathematics/ternary/example"` to a generator command to emit its program and target. A fixed operation and width always generate the same rule bodies; only the initial digit facts change with the operands. Expected values do not influence the program or its width.

## Readable digit rules

The [small digit library](../ternary/digit.particle) makes the arithmetic cases inspectable. Representative complete rules are:

```text
[Add.Zero.One.Two] TritZero.CarryOne
[Multiply.Two.Two] TritOne.CarryOne
[Subtract.LeftZero.RightTwo.BorrowOne] TritZero.BorrowOne
[Select.LeftOne.RightTwo.ChoiceOne] TritTwo
```

Three contributions 0 + 1 + 2 give digit zero and carry one. Two times two gives digit one and carry one: 1 + 3 = 4. Subtraction computes a digit and an outgoing borrow; selection chooses a digit from positive choice data. These examples can run independently under the digit library. Their tests cover all input values and permutations of the commutative operations.

The word generator wires the same digit computations using distinct port names. Ports maintain the association between operations in an orderless state. The standalone library is not a constant-size interpreter for arbitrary words, and its commutative patterns do not by themselves route unknown operands through a circuit.

## How the operations compose digit computations

**Addition:** put the two input contributions in each column and propagate carries. A local reduction replaces two or three digits by a digit at the same position and a carry at the next. The result has width plus one digits.

**Subtraction:** run borrow chains in both directions. The final borrow selects the nonnegative magnitude; a separate NegativeZero or NegativeOne fact records its sign. Equal inputs give positive zero. Inputs remain natural numbers.

**Multiplication:** multiply every pair of input digits. A product contributes its low digit to column i + j and its carry to column i + j + 1. Reduce the columns with the shared carry machinery. The complete result has twice the input width. Its size suffices because both operands are smaller than 3 to that width; any carry beyond the output width is mathematically zero for valid inputs.

**Division:** process dividend digits from most to least significant. Form T = 3R + the next digit. Attempt subtraction of the divisor twice, keeping a subtraction only when its positive borrow result permits it. The number of successful subtractions is the next quotient digit. If R is initially below a positive divisor, T is below three times the divisor, so two attempts suffice. Intermediate subtraction uses one extra digit; truncation happens only after the quotient digit is resolved.

For a zero divisor, explicit digit rules produce UndefinedOne, quotient zero, and the original dividend as remainder. That is a total error-payload convention, not a mathematical quotient. For a nonzero divisor the result carries UndefinedZero and satisfies left = quotient × right + remainder with remainder smaller than the divisor.

```sh
bazel run -c opt //mathematics/arithmetic:word -- \
  --operation subtract --left 123 --right 1500 --expected -1377

bazel run -c opt //mathematics/arithmetic:word -- \
  --operation divide --left 1500 --right 0 \
  --expected 0 --remainder 1500 --undefined
```

## Notation and limits

Photonic's grammar is unchanged. A complete ternary word contains one ordinary TritNZero, TritNOne, or TritNTwo concept at each position. Quotients and remainders use separate prefixes. Explicit zero values are necessary: an omitted output is not accepted as a computed zero. Exact targets include all digits and applicable sign/definedness flags.

The sparse carry library also supports `3^0.3^1.3^1`, representing seven. Each power spelling is an ordinary atom name with meaning supplied by library rules. The caret is not an exponent operator. `Power(3,1)` continues to expand into two coherences; it is not an ordered base/exponent constructor.

The external generator accepts decimal, `0t` ternary, `0b` binary, `0o` octal, and `0x` hexadecimal input, with optional underscores between digits. These forms all encode ternary circuit inputs. For example:

```sh
bazel run -c opt //mathematics/arithmetic:word -- \
  --operation multiply --left 0t2001120 --right 0t11120 --expected 184_500
```

In a `.particle` file, `0t2001120` is just an ordinary concept name. No input-format prefix adds a language operation. The old `//mathematics/binary:word` command has moved to `//mathematics/arithmetic:word`; its width now counts ternary digits.

Width is inferred from the largest operand, with a minimum of one, or set explicitly with `--width`. Supported input width is 1 through 20 trits, corresponding to values through 3486784400. Products use up to 40 trits. No output is silently wrapped to fit a smaller target. Larger programs may need more execution resources; this is a finite-width family, not arbitrary precision.

A separately recorded maximum-width command reached 3486784400² = 12157665452083360000 in 1218 events and 1026119 work steps, taking 21839 ms in one local optimized run. That is operational evidence for this case, not a uniform performance bound.

## Optimizations and measurements

The builder propagates possible digit values independently of the actual operands. It emits only applicable truth-table rows: borrow and choice inputs have two values even in a ternary circuit. It omits zero-only product carries from column reduction and removes gates with no path to a requested output. Both radices use the same implementation.

The runtime shares immutable compiled programs across direct-path steps. A scope index selects rules by a required symbol; a further presence check avoids allocating searches that cannot match the current derived view. It uses symbols from the view's target, so inferred abstractions still enable applications at their concrete sources. This is a necessary-condition filter, not a negative language premise. Empty patterns, live rules, captures, multiplicity, and actual multi-coherence matching retain their existing treatment.

Median of five local optimized samples, for inputs 1500 and 123. Timing includes parsing, initialization, path search, and report construction; source construction and final destruction are excluded. Layouts and operations were measured in consecutive batches, so timing may include ordering or thermal effects.

| Operation | Ternary rules | Ternary events | Ternary work | Ternary time | Binary time |
| --- | ---: | ---: | ---: | ---: | ---: |
| Add | 159 | 21 | 1347 | 1352 µs | 4124 µs |
| Subtract | 403 | 36 | 2302 | 3038 µs | 7437 µs |
| Multiply | 1777 | 152 | 19230 | 51359 µs | 500773 µs |
| Divide | 3630 | 268 | 22538 | 73100 µs | 126936 µs |

Ternary won these four comparisons. This does not establish superiority across inputs; the [sparse addition comparison](../ternary/README.md) includes a case where binary won. Ternary multiplication has more rule rows than binary here, but fewer events and substantially less matching work on the selected path.

Balanced ternary-radix reduction took 89241 µs, with 1795 rules and 39818 work steps for the same multiplication. Column reduction remains the default. Balanced reduction here describes circuit layout, not a signed balanced-ternary digit representation. No parallel speedup is claimed: the path strategy is serial.

```sh
bazel run -c opt //mathematics/arithmetic:benchmark
bazel test -c opt //...
```

## What remains open

All pairs of two-trit operands are tested for all four operations, including wrong answers, remainders, and flags. One-trit addition and multiplication also receive exhaustive correct/incorrect target checks. Tests cover larger inputs, maximum-width addition/subtraction, fixed-width rule invariance, and the separate digit library. Runtime tests cover source inference, lexical scope, code values, multi-coherence joins, budgets, scheduling, and irrelevant-rule indexing.

These are concrete checks plus written invariants, not universally quantified arithmetic certificates. Path success proves a witness; a failed path remains Unknown and other paths are not exhausted. Reports are inspection artifacts, not independently replayable certificates.

The [stream investigation](stream.md) now includes a twelve-rule ternary successor, validated through ordered direct execution traces. Arithmetic over arbitrary unknown stream tails and the construction of functional result numerals are still unresolved. There is no hidden wildcard, structural binder, or native number handler filling that gap. The current deliverable is a working, measured ternary word implementation with ordinary-rule semantics.
