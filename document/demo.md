# Local native ternary expressions

Each small demo is a self-contained Photonic input file. Its `Stage` and `Inspect` controls belong to that program, so no case-name prefix is needed. The expression library remains a separate dependency.

| Native input | Ternary expression | Result |
| --- | --- | --- |
| [Addition](../mathematics/ternary/demo/addition.wave) | `12 + 2` | `21` |
| [Multiplication](../mathematics/ternary/demo/multiplication.wave) | `12 * 2` | `101` |
| [Division and subtraction](../mathematics/ternary/demo/division.wave) | `21 / 2 - 1` | `2` |

Every numeral in this table is base three, most significant trit first. In decimal these are `5 + 2 = 7`, `5 * 2 = 10`, and `7 / 2 - 1 = 2`. Division truncates toward zero.

The [webbook visualizer](../index.html#expression) shows the recorded input tape and intermediate results. Selecting a demo plays a saved execution; it does not evaluate new browser input. The source panel shows the actual native file. Displayed numeral fields are success markers emitted after checking returned trits; they are not a general-purpose printer.

Run a localized program:

```sh
bazel test -c opt //mathematics/ternary:demo.addition.check --test_output=all
bazel test -c opt //mathematics/ternary:demo.multiplication.check --test_output=all
bazel test -c opt //mathematics/ternary:demo.division.check --test_output=all
```

These return `Number.([Trit] 21)`, `Number.([Trit] 101)`, and `Number.([Trit] 2)` respectively. Each is an independent program; their local helper declarations must not be concatenated into one root scope. Fields such as `([Digit] 2)` are captured rule values, so wrapping input declarations in a new scope is not equivalent to importing the library into that scope.

The [combined program](../mathematics/ternary/demo.wave) still runs all three sequentially in one program. Its controls retain case namespaces to distinguish their protocols; its output no longer uses a `Result` prefix.

Run from the repository root:

```sh
bazel test -c opt //mathematics/ternary:demo.check --test_output=all
```

Success reports:

```text
Reached; Addition.([Trit] 21), Multiplication.([Trit] 101), Division.([Trit] 2)
```

To run the executable directly:

```sh
bazel run -c opt //mathematics/ternary:demo -- obsidian \
  --target "$PWD/mathematics/ternary/demo.particle" --path \
  --steps 100000000 --states 65536 --cells 16384 \
  --frames 2048 --coherences 1024 --records 100000000
```

## Reading the input

The readable expressions above describe the input; they are not literal Photonic string syntax. The native library accepts a linked token tape. For `12 + 2`, its tokens are:

```text
1 2 Add 2
```

The program builds that tape by prepending tokens, starting with the last one:

```photonic
Push.([Digit] 2).Zero, Stage.1
[Built,Stage.1] (Push.Add) (Forget.Stage.2)
[Clean.Stage.2] Stage.2
[Built,Stage.2] (Push.([Digit] 2)) (Forget.Stage.3)
[Clean.Stage.3] Stage.3
[Built,Stage.3] (Push.([Digit] 1)) (Forget.Stage.4)
[Clean.Stage.4] Stage.4
```

`Function.Expression` then evaluates the tape natively. The program reads the returned trits, checks the expected result and end of the numeral, and starts the next calculation. Result trits are read least significant first: `21` returns `1`, then `2`. The displayed result fields are success markers emitted only after those checks, not a general-purpose numeral printer. If you change an expression, update its expected-result checks too.

The three calls run sequentially in the same active frame, so their internal registers and cells do not overlap. The [Bazel target](../mathematics/ternary/BUILD.bazel) includes the `:formula` library; no Rust expression parser or arithmetic compiler is involved. See the [expression interface](expression.md#expression-interface) for negative values, parentheses, and reusable numeral results.

## Recorded data

Regenerate the smaller demos and original example after changing their input, library, or recorder:

```sh
bazel run -c opt //mathematics/ternary:demo.record > document/demo.record.js
bazel run -c opt //mathematics/ternary:record > document/expression.record.js
```

The recorder checks each supplied target, walks the native input tape to verify its tokens, and decodes each recorded positive intermediate numeral. These selected demos have positive intermediate results. Signed arithmetic is covered separately by the native library tests. Decimal conversion in the viewer and recorder formats produced trits; it does not evaluate the arithmetic expression.

The `demo.record.check` and `performance` targets compare regenerated records byte-for-byte with the files served by the book. Browser tests cover every selector option, operation navigation, source visibility, input tokens, result trits, and the narrow-screen layout. These checks protect the documented examples; they do not prove every claim about every possible Photonic program.
