# Ternary streams

Status: a fixed-rule successor produces the correct digit sequence on the tested direct paths. It does not yet construct a reusable numeral value. The [word implementation](README.md) remains the arithmetic implementation for exact numerical targets.

## Representation

Position belongs to the stream, not to a numbered atom. Digits arrive least significant first; End explicitly terminates the input. The mathematical interpretation is:

```text
value(end)         = 0
value(digit, tail) = digit + 3 × value(tail)
```

These equations describe the encoding; they are not Photonic syntax or native operations. Eighteen is Zero, Zero, Two, End. Twenty is Two, Two, One, End.

The [example input](../ternary/stream.wave) expresses twenty with existing scoped rules:

```text
Read.Carry
[Read] (
    Two
    [Next] (
        Two
        [Next] (
            One
            [Next] End
        )
    )
)
```

The nested Read/Next definitions are numeral data. Their size grows with the digit count. The arithmetic library is fixed.

## Successor

The complete [successor library](../ternary/successor.particle) contains twelve ordinary rules:

```text
[Carry.Zero] Copy.([Write] One)
[Carry.One] Copy.([Write] Two)
[Carry.Two] Carry.([Write] Zero)

[Copy.Zero] Copy.([Write] Zero)
[Copy.One] Copy.([Write] One)
[Copy.Two] Copy.([Write] Two)

[Carry.End] Finish.([Write] One)
[Copy.End] Done

[([Write] Zero)] Next
[([Write] One)] Next
[([Write] Two)] Next
[Next.Finish] Done
```

Carry requests an increment. Zero becomes One; One becomes Two. Both switch to Copy, which preserves the remaining digits. Two becomes Zero and leaves Carry active for the next digit. Carry at End emits a final One. Every termination rule requires positive evidence.

Each output digit is an ordinary rule value, such as `[Write] Zero`. The three meta rules consume those whole values and acknowledge them with Next. This separates emitted digits from input digits without new grammar, numbered ports, numerical primitives, or special matching. There is no Write request in this protocol, so the values are consumed as data.

For twenty, the direct execution emits Zero, Zero, Two: twenty-one. The result order is in the execution trace. Dotting those three atoms together would discard their order and would not represent twenty-one under this encoding.

## Run

From the repository root:

```sh
directory=$(mktemp -d)
cat mathematics/ternary/stream.wave mathematics/ternary/successor.particle > "$directory/program.wave"
bazel run -c opt //system:command -- obsidian "$directory/program.wave" \
  --target "$PWD/mathematics/ternary/done.particle" \
  --path --json --steps 100000 --states 1024 --cells 4096 --frames 128
```

The JSON event list includes acknowledgements for Zero, Zero, Two in that order. The terminal target is Done; it is not a numeral certificate. The three acknowledgement rules consume the emitted code values, so those values do not remain as a result in the final configuration.

## Validation and cost

The [regression suite](stream.rs) checks every input from zero through 242, unchanged suffixes, leading zeros, carries through one to forty Two digits, and the successor of the largest unsigned 64-bit integer. It checks the actual acknowledgement sequence against an independent numerical oracle. It also verifies that the twelve arithmetic declarations are unchanged across different input widths and values.

On the tested direct paths, a numeral with n digits takes 3n + 2 events, or 3n + 4 when a final carry appends One. The twenty-to-twenty-one example takes 11 events and 171 work steps. Forty Two digits take 124 events and 4934 work steps in the recorded run.

Constant arithmetic code and linear event count do not imply constant memory or linear total execution cost. The forty-digit run retains up to 820 held occurrences across lexical frames. Input syntax depth, retained contexts, matching, canonicalization, and stored path history remain costs. Earlier smaller cell limits changed which successor paths were available; increasing the work budget alone did not address that constraint.

## Remaining contract

This is a direct-path prototype, not a proof that every permitted inference produces one canonical stream. Ancestor Next rules remain lexically available; there is no implicit shadowing or rule priority. Source inference can also bypass intermediate emissions. The language treats those emissions as ordinary states, not externally guaranteed effects. Reaching Done alone therefore cannot prove the output sequence.

The next requirement is to retain an ordered result that another ordinary rule computation can consume, while preserving operand association and accounting for every digit under source inference. Demonstrate that contract for successor before adding two-stream addition, multiplication, or division. Do not introduce a native numeral, hidden binding, or changed parenthesis semantics to fill the gap.

`Power(3, Base.2).Coefficient(2)` remains shared-prefix expansion into two flat coherences. It does not preserve a base/exponent/coefficient record. The recursive direction removes explicit positions, but spelling alone does not supply the missing composition contract.
