# Native expressions

The expression library evaluates signed integer expressions using ordinary Photonic declarations. It implements binary addition, subtraction, multiplication, division, unary minus, precedence, and parentheses. Rust neither parses the arithmetic expression nor constructs its arithmetic graph. Bazel assembles the checked-in Photonic library and input into one program for the existing runtime.

```text
50 × 3 ÷ 2 + 4 − 1
1212₃ × 10₃ ÷ 2₃ + 11₃ − 1₃ = 2220₃ = 78₁₀
```

## Run

From the repository root, with Bazel as the only required installed build tool:

```sh
bazel test -c opt //mathematics/ternary:infix.check --test_output=all
bazel test -c opt //mathematics/ternary:all --test_output=errors
```

The [input fixture](../mathematics/ternary/infix.wave) constructs a token tape, calls `Function.Expression`, reads the four result trits in least-significant-first order, and reaches exactly `Done.Zero`. The result check consumes the numeral; it is separate from expression evaluation. The evaluator itself returns a reusable linked numeral. Remove the inspection declarations when retaining that result for further computation.

To inspect the complete execution:

```sh
bazel run -c opt //mathematics/ternary:infix -- obsidian \
  --target "$PWD/mathematics/ternary/result.particle" --path \
  --steps 100000000 --states 65536 --cells 16384 \
  --frames 2048 --coherences 1024 --records 100000000 --json
```

The [webbook](../index.html#expression) shows the four intermediate results decoded from that native execution. Regenerate its small record with:

```sh
bazel run -c opt //mathematics/ternary:record > document/expression.record.js
```

The recorder uses Bazel's pinned Node executable to run the existing Photonic engine, select operation-completion events, and format the produced digits. It does not evaluate an expression. The browser displays this saved record; its existing live sandbox has smaller fixed execution budgets.

## Expression interface

`Function.Expression` accepts a linked token tape. The token alphabet is the three digit fields `([Digit] 0)`, `([Digit] 1)`, `([Digit] 2)`, and `Add`, `Subtract`, `Multiply`, `Divide`, `Open`, `Close`. Successive digit tokens form a ternary integer, most significant first. `Open` and `Close` are parentheses. `Subtract` is unary when an operand is expected. Leading zeroes are accepted.

For example, the tape for the demonstration reads:

```text
1 2 1 2 Multiply 1 0 Divide 2 Add 1 1 Subtract 1
```

This line describes the ordered tape; bare numeric atoms have no built-in numerical meaning. A native constructor request `Push.([Digit] 2)` prepends a digit to a head, and `Push.Multiply` prepends an operator. `Built` acknowledges construction. `Zero` terminates a tape. Build in reverse token order, then replace `Built` with `Function.Expression`. The fixture's `Stage` labels sequence this input construction. They encode this particular finite input, not a table of permitted widths, arithmetic answers, or expression depths. The library contains no indexed stage table.

A complete two-token numeral, using the same library, can be supplied as:

```photonic
Push.([Digit] 2).Zero, Stage.1
[Built,Stage.1] (Push.([Digit] 1)) (Forget.Stage.2)
[Clean.Stage.2] Stage.2
[Built,Stage.2] Function.Expression
```

It evaluates the ternary literal `12`. The constructor's control copy must call `Forget` before advancing. This removes its extra head reference while retaining the actual tape and cells.

Success returns `Return.Expression.Number.Positive` or `Return.Expression.Number.Negative`, together with a numeral head and its cells. Zero is always positive. Errors return `Return.Expression.Error.Syntax`, `Return.Expression.Error.Stack`, or `Return.Expression.Error.Divisor`; the error paths clean up the remaining tapes. The stack error describes malformed postfix input at the lower evaluator interface.

Multiplication and division take precedence over addition and subtraction. Binary operators associate left to right. Unary minus binds first and may repeat or precede parentheses. The [parenthesis fixture](../mathematics/ternary/parenthesis.wave) evaluates `12₃ × (21₃ + 2₃) = 1200₃`.

Division is integer division truncated toward zero: `100₃ / 2₃ = 11₃`, and `-100₃ / 2₃ = -11₃`. Fractions, floating-point arithmetic, powers, and named functions are not part of this interface. This is a numeric-domain choice, not a claim that Photonic cannot represent them. In particular, signed numerator/positive denominator pairs and these integer operations provide a native route to rational arithmetic; that additional numeric layer is not implemented here.

## Representation

Numerals use least-significant-first linked trits:

```text
value(Zero) = 0
value(node(digit, tail)) = digit + 3 × value(tail)
```

`Zero` is the empty numeral. A nonempty head contains `Head`, a captured seal, and callable node methods. Its cell contains `Cell`, the same captured seal, and the tail. Each constructor enters a fresh existing scope; the captured scope distinguishes its seal from every other live node's seal. No new atom naming convention, structural binder, generated rule, or language feature supplies identity.

`Read` joins the head with its own cell, removes the node's methods, and returns `Yield.([Digit] d)` with the tail. `Read.Zero` returns `Yield.End.Zero`. Token nodes use the same mechanism. `Forget` removes a head reference without reading or deleting the cells. It is used only to clean temporary aliases created when a rule broadcasts its remainder. `Function.Erase` walks a chain and consumes its cells.

A numeral is the head **and its cells**, not the head particle alone. Operations consume their inputs. Use `Function.Copy` for two independently consumable magnitude copies; broadcasting only the head creates aliases, not two numerals. Copy reverses the input once and constructs both outputs in their final orientation. `Function.Reverse` consumes a chain and constructs its reversal. Normalization removes high zeroes and represents zero by the empty numeral.

Keep all cells and their controlling head in the same active frame. These register protocols serialize calls within that frame; overlapping independent evaluator calls in one frame are outside the interface. In particular, moving only a head into a generic `Invoke` body does not move its cells. Arbitrarily nested expressions are handled by the interpreter's own linked stacks, without recursively invoking independent evaluators. Multiplexing independent calls would need additional request association, for which the same captured-identity technique provides a library-level direction; no impossibility is asserted.

## Layers

| Layer | Source | Contract |
| --- | --- | --- |
| Node construction | [chain](../mathematics/ternary/chain.particle) | Finite token alphabet; fresh captured identity per cell |
| Traversal | [reverse](../mathematics/ternary/reverse.particle), [copy](../mathematics/ternary/copy.particle), [erase](../mathematics/ternary/erase.particle) | Reverse or discard finite token chains; duplicate trit magnitudes |
| Canonical magnitude | [trim](../mathematics/ternary/trim.particle), [normalize](../mathematics/ternary/normalize.particle) | Remove high zeroes |
| Natural arithmetic | [add](../mathematics/ternary/add.particle), [subtract](../mathematics/ternary/subtract.particle), [multiply](../mathematics/ternary/multiply.particle), [divide](../mathematics/ternary/divide.particle) | Two magnitude operands; canonical result |
| Column arithmetic | [column](../mathematics/ternary/column.particle), [borrow](../mathematics/ternary/borrow.particle), [complement](../mathematics/ternary/complement.particle) | Shared addition/subtraction traversal; independently associated operand reads; negative magnitude recovery |
| Signed arithmetic | [difference](../mathematics/ternary/difference.particle), [integer](../mathematics/ternary/integer.particle) | Sign handling over natural operations |
| Value stack | [split](../mathematics/ternary/split.particle), [join](../mathematics/ternary/join.particle) | Extract or prepend signed numeral records |
| Postfix evaluation | [evaluate](../mathematics/ternary/evaluate.particle) | Evaluate literals, unary negation, and binary operators |
| Infix conversion | [convert](../mathematics/ternary/convert.particle) | Shunting-yard conversion with a linked operator stack |
| Public function | [expression](../mathematics/ternary/expression.particle) | Compose conversion and evaluation |

The natural binary functions take two coherences labelled `Operand.Left` and `Operand.Right`, each carrying a magnitude head, plus a `Function.Add`, `Function.Subtract`, `Function.Multiply`, or `Function.Divide` request. Their cells remain in the same configuration. Add and Multiply return `Return.Add` and `Return.Multiply`; Subtract returns `Return.Subtract.Number` or `Return.Subtract.Error.Underflow`. Divide returns separate `Return.Divide.Quotient` and `Return.Divide.Remainder` heads, or `Return.Divide.Error.Divisor`.

Signed operands additionally carry exactly one `Positive` or `Negative` label. Call `Function.Integer.([Operation] Add)` with those operands, substituting the desired operator in the field. It returns `Return.Integer.Number` with the sign and magnitude, or `Return.Integer.Error.Divisor`. Its division interface discards the natural remainder after computing the quotient.

`Function.Evaluate` is the lower postfix interface. A literal is `Literal`, followed by its most-significant-first digit tokens and `Mark`; operators follow their operands. `Negate` is unary. Results use `Return.Evaluate.Number` with sign and magnitude. This permits arbitrary finite expression composition without host-side evaluation. The infix converter emits this representation natively.

## Why width and nesting need no table

The constructor creates another cell for another trit using the same declarations. Traversal advances to its tail using the same declarations. Thus a finite list can be extended by another node without adding a digit position or changing the library. The finite cases enumerate trit values, carry/borrow states, token kinds, operator precedence, and signs. They never enumerate whole operands, widths, or nesting depths.

Addition and subtraction share one column engine, padding a finished operand with zero until both finish. Each column starts independently associated left and right reads; reduction waits for both digits and the carry or borrow. Each step preserves `left + right + carry = digit + 3 × nextCarry`, or `left − right − borrow = digit − 3 × nextBorrow`. The output buffer is normalized and reversed. A final borrow produces natural underflow. For signed difference, the n-trit buffer instead represents `3ⁿ + left − right`. Complementing each trit and adding one recovers `right − left`, without preserving either operand or retrying subtraction.

Multiplication reads the multiplier most significant first. Each step computes `product = 3 × product + digit × factor`, requiring zero, one, or two additions. Long division maintains `0 ≤ remainder < divisor`. Appending a dividend trit gives `3 × remainder + digit < 3 × divisor`, so at most two successful subtractions determine the next quotient trit. Copies preserve the remainder when a trial subtraction underflows. Division by zero is detected before this loop.

Expression traversal uses linked operator and value stacks. Parentheses push and pop stack entries rather than selecting a predeclared depth. These constructions supply a native path around the old positional jump table. The older fixed-width circuit builder remains a separate interface; its bounds do not apply here.

An actual execution is finite and resource-bounded. Memory, frame, coherence, configuration, record, and work budgets can stop a run. The representation and algorithms have no fixed trit or expression-depth parameter; this is not a claim of infinite physical memory or constant runtime cost.

## Validation boundary

The native checks cover all four operations, operator precedence and association, parentheses, repeated unary minus, positive and negative intermediates, zero normalization, borrow and carry propagation, nonzero remainders, zero divisors, malformed syntax and stacks, 21 nested parentheses, and a 21-trit addition that produces 22 trits. Exact final configurations include cleanup of the constructed cells and callable code.

The arithmetic checks are direct-path reachability checks supported by written algorithm invariants. The small column-protocol checks also explore their complete finite state spaces, including rejection of exchanged operand roles and an unrelated operation's completion marker. These checks do not establish whole-evaluator confluence, prove all incorrect arithmetic results unreachable, or provide a machine-checked universal arithmetic theorem. A failed bounded search remains Unknown. No theoretical impossibility follows from it.

## Optimization and concurrency

Reads carry their continuation directly on the returned tail, avoiding a temporary alias and its cleanup. Operator roles are protected inside fields: an incoming operator and a popped operator cannot exchange roles merely because their names coexist in an orderless particle. Function completion uses an explicit acknowledgement field where an outer expression could otherwise supply the same bare labels.

The column engine starts its left and right operand reads independently, including the first column. Copy completion also returns its left and right results independently; neither completion waits for the other result to be consumed. This is protocol-level concurrency; the recorded direct-path executor still schedules individual events serially. Complete independent subexpressions are not yet scheduled concurrently, and calls sharing one frame remain serialized. No multicore speedup or globally optimal evaluator is claimed.

The performance test runs the complete example, checks its decoded intermediate values, and rejects runs exceeding 10,200 events or 3,500,000 work steps. The initial implementation required 13,850 events and 5,741,452 work steps. These are reproducible engine counters for this workload, not universal complexity bounds or wall-clock speedups.

| Complete example | Initial | Previous | Current |
| --- | ---: | ---: | ---: |
| Events | 13,850 | 10,686 | 10,017 |
| Work steps | 5,741,452 | 3,843,442 | 3,411,783 |

This pass reduces events by 6.3% and work by 11.2% relative to the previous implementation. Constructors introduce the cell directly in its captured scope. Token reads return bare token values directly; digit reads retain the root conversion that preserves the digit field's capture. Empty-tail termination consumes `Zero` explicitly instead of passing it through head cleanup. Roles, fields, and function boundaries remain explicit.

A shared constructor that stored every token as cell data passed the functional checks but raised this example's work to 4,632,304. It was rejected. Fewer declarations alone do not imply less matching work.

```sh
bazel test -c opt //mathematics/ternary:all //tool:check //tool:browser \
  --test_output=errors
```

The remaining costs include linked-cell construction, magnitude copying during multiplication and division, repeated traversal for normalization, and interpreter-stack transfers. Removing those costs or parallelizing complete subexpressions requires further measured library design; the current improvements do not establish perfection.
