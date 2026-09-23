# Photonic standard library

The standard library is written entirely in Photonic. Rust loads the sources and checks their behavior; every operation runs through ordinary rules. Thirteen packages cover calling conventions, scalar tables, finite collections, linked storage, unbounded arithmetic, and expression evaluation. Any combination of them can be loaded together.

```sh
bazel test -c opt //library/...
```

## Principles

1. **One namespace per type.** Every operation is named `<Type>.<Verb>`: `Boolean.Not`, `Ternary.Add`, `Natural.Divide`, `Expression.Evaluate`. The namespace belongs to exactly one package. Only the core combinators `Identity` and `Compose` are bare.
2. **One calling vocabulary.** A request carries `Function`; its answer carries `Return`. Scoped calls use `Invoke`; linked calls tag each answer with the operation that produced it.
3. **Collision-free by construction.** `//library:test` proves that no root rule of one package can match the input of another package's rule, then runs checks with every package loaded at once.
4. **Explicit values.** Roles travel as fields such as `([Digit] 2).([Carry] 1)`. Alternatives are variants of the answer, and failures are `Error.<Kind>`.
5. **Generic storage, declared alphabets.** A chain stores any symbol. Each alphabet states how chains drop, reverse, and erase its symbols.
6. **One responsibility per file, one target per file.** Programs depend on exactly what they use.

## Packages

| Package | Namespace | Targets |
| --- | --- | --- |
| [function](function/) | `Invoke`, `Identity`, `Compose` | `invoke`, `identity`, `compose` |
| [boolean](boolean/) | `Boolean` | `not`, `and`, `or`, `equal` |
| [ternary](ternary/) | `Ternary` | `add`, `sum`, `multiply`, `successor`, `compare`, `subtract`, `select` |
| [binary](binary/) | `Binary` | `sum`, `multiply` |
| [carry](carry/) | `Carry` | `combine`, `evaluate` |
| [collection](collection/) | `Pair`, `Empty` | `produce`, `copy`, `repeat`, `broadcast`, `unpack`, `choose`, `map`, `gather`, `reduce` |
| [selection](selection/) | `Selection` | `filter`, `check`, `count`, `reduce` |
| [field](field/) | `Field` | `pack`, `unpack` |
| [stream](stream/) | `Stream` | `successor` |
| [chain](chain/) | `Chain` | `cell`, `reverse`, `erase` |
| [natural](natural/) | `Natural` | `digit`, `copy`, `trim`, `normalize`, `successor`, `complement`, `column`, `add`, `borrow`, `subtract`, `difference`, `multiply`, `divide` |
| [integer](integer/) | `Integer` | `add`, `subtract`, `multiply`, `divide`, `result` |
| [expression](expression/) | `Expression` | `token`, `split`, `join`, `parse`, `execute`, `evaluate` |

Targets carry their dependencies, so `deps = ["//library/natural:divide"]` loads the chain, the digit alphabet, copying, subtraction, and normalization it needs. The scalar packages depend on `//library/function:invoke`. The layering is:

```
function ── boolean, ternary, binary, carry, field
         └─ collection ── selection
chain ── natural ── integer ── expression
stream
```

## Scoped invocation

`Invoke` opens a scope, activates `Function` inside it, and releases whatever follows `Return`:

```
Invoke.Boolean.And.True.False                      → False
Invoke.Ternary.Add.2.2                             → ([Digit] 1).([Carry] 1)
Invoke.Ternary.Compare.([Left] 0).([Right] 2)      → Less
Invoke.Carry.Combine.([Left] Kill).([Right] Generate) → Generate
Invoke.Field.Pack.([Position] 1).([Value] 2)       → ([1] 2)
Invoke.Identity.Payload                            → Payload
```

An implementation answers with `[Function.<Type>.<Verb>.<arguments>] Return.<result>`. Unmatched operands follow the ordinary remainder law, so `Return` alone never certifies a complete result; consumers match the payload they require.

`Compose` sequences two supplied callables and keeps both:

```
Invoke.Compose.Seed.([First.Seed] Return.Bud).([Second.Bud] Return.Flower)
    → Flower.([First.Seed] Return.Bud).([Second.Bud] Return.Flower)
```

## Callbacks and pipelines

A callback is a descriptor rule whose output names an operation: `([Each] Boolean.Not)`, `([Operation] Boolean.And)`, `([First] Function.Boolean.Not)`. Each implementation owns the dispatch rule that consumes its descriptor, so adding an operation never edits the collection protocol.

The collection package implements a finite pipeline over pairs:

```
Invoke.Pair.Copy.([Value] True).Map.([Each] Boolean.Not).Reduce.([Operation] Boolean.And) → False
Invoke.Pair.Repeat.([Value] 1).([Each] Pair.Produce).Reduce.([Operation] Ternary.Add)     → ([Digit] 2).([Carry] 0)
Invoke.Pair.Copy.([Value] True).Map.([Each] Selection.Filter).Reduce.([Operation] Selection.Count) → 2
Invoke.Pair.Choose.False.([Left] True).([Right] False)                                   → False
Invoke.Empty.Reduce.([Operation] Boolean.And)                                            → True
```

`Pair.Copy` produces fresh values in two scopes, `Pair.Broadcast` shares one inherited value, and `Map` runs `Each` on both sides independently. `Gather` and `Reduce` join both completed payloads, never completion markers alone. Each operation declares its own identity for `Empty.Reduce`, as `Boolean.And` answers `True`. Payload shapes are finite: Booleans and trits for `Reduce`, Booleans for `Gather`, and `Keep`/`Discard` selections through `//library/selection:reduce`.

## Linked calls

Linked values span several coherences in one frame, so linked operations run in the caller's frame rather than inside `Invoke`. A unary request sits beside its operand's handle; a binary request sits beside `Operand.Left` and `Operand.Right` coherences. The answer is tagged with the operation that produced it:

| Request | Operands | Answer |
| --- | --- | --- |
| `Function.Chain.Reverse` | beside the chain | `Return.Chain.Reverse` |
| `Function.Chain.Erase` | beside the chain | `Return.Chain.Erase` |
| `Function.Natural.Copy` | beside the numeral | `Return.Natural.Copy.Left`, `Return.Natural.Copy.Right` |
| `Function.Natural.Normalize` | beside the numeral | `Return.Natural.Normalize` |
| `Function.Natural.Successor` | beside the numeral | `Return.Natural.Successor` |
| `Function.Natural.Complement` | beside the numeral | `Return.Natural.Complement` |
| `Function.Natural.Add` | `Operand.Left`, `Operand.Right` | `Return.Natural.Add` |
| `Function.Natural.Subtract` | `Operand.Left`, `Operand.Right` | `Return.Natural.Subtract.Number`, `Return.Natural.Subtract.Error.Underflow` |
| `Function.Natural.Difference` | `Operand.Left`, `Operand.Right` | `Return.Natural.Difference.Positive`, `Return.Natural.Difference.Negative` |
| `Function.Natural.Multiply` | `Operand.Left`, `Operand.Right` | `Return.Natural.Multiply` |
| `Function.Natural.Divide` | `Operand.Left`, `Operand.Right` | `Return.Natural.Divide.Quotient` and `Return.Natural.Divide.Remainder`, or `Return.Natural.Divide.Error.Divisor` |
| `Function.Integer.<Verb>` | `Operand.Left.<Sign>`, `Operand.Right.<Sign>` | `Return.Integer.<Verb>.Positive`, `Return.Integer.<Verb>.Negative`, `Return.Integer.Divide.Error.Divisor` |
| `Function.Expression.Evaluate` | beside the token tape | `Return.Expression.Evaluate.Positive`, `Return.Expression.Evaluate.Negative`, `Return.Expression.Evaluate.Error.Syntax`, `.Error.Stack`, `.Error.Divisor` |

Numerals store base-three digits least significant first. Arithmetic answers carry no leading zeros; integers carry `Positive` or `Negative`, zero is always `Positive`, and integer division truncates toward zero.

## Chains

A chain handle is `Zero` or a `Head` coherence carrying a private seal and its methods; its cells remain elsewhere in the same frame.

| Request beside a chain | Answer |
| --- | --- |
| `Push.<symbol>` | `Built` beside the extended chain |
| `Read` | `Yield.<symbol>` beside the tail, or `Yield.End.Zero` |
| `Forget` | `Clean` once this reference is dropped |

A join hands its unmatched remainder to every output, so moving a chain means forgetting it everywhere else. The move idiom sends the chain to its next use and resumes once the old reference is clean:

```
Push.([Digit] 1).Zero, Stage.1
[Built, Stage.1] (Push.([Digit] 2)) (Forget.Stage.2)
[Built, Clean.Stage.2] (Operand.Left) (Forget.Stage.3)
```

`Push` is symbol-agnostic. An alphabet declares one `Drop` rule per symbol so a new head can forget its predecessor, plus the hooks that let `Reverse` and `Erase` traverse its symbols. The digit alphabet reads:

```
[Drop.([Digit] 0)] Forget
[Yield.([Digit] 0).Reverse.Wait] (Reverse.Left) (Forget.Reverse.Write.([Digit] 0))
[Clean.Reverse.Write.([Digit] 0), Reverse.Right] (Push.([Digit] 0)) (Forget.Reverse.Await)
[Yield.([Digit] 0).Erase.Wait] Read.Erase.Wait
```

`//library/natural:digit` declares the digits and `//library/expression:token` declares `Mark`, `Literal`, `Add`, `Subtract`, `Multiply`, `Divide`, `Open`, `Close`, `Negative`, and `Negate`. The `//library/chain:cell.check` test stores a caller-defined symbol with nothing but its own `[Drop.Item] Forget`.

## Streams

A stream is a nested body that reveals one digit whenever `Next` appears. Place `Function.Stream.Successor` where the first digit will arrive:

```
Seventeen.Function.Stream.Successor
[Seventeen] (2 [Next] (2 [Next] (1 [Next] End)))
```

The successor writes each output digit as a `([Write] d)` value, acknowledges it with `Next` to advance the stream, and answers `Return.Stream.Successor`. Ancestor bodies keep their `Next` rules visible, so streams rely on the direct path's preference for the nearest body.

## Guarantees

`//library:test` holds the library's composition contract:

- `isolation::boundary` parses every package and fails if any root rule could match the input of a root rule in a different package. It rejects the collisions this layout removed: natural `[Function.Add]` matching ternary `Function.Add.1.2`, ternary multiplication matching binary requests, `[Function.Compose]` matching carry composition, the stream's `[Carry.0]` matching the column engine, and `[Invoke]` matching internal states once named `Multiply.Invoke`.
- `isolation::vocabulary` requires single-word concepts everywhere and at most two input and output coherences in the scalar packages.
- `composition` runs scalar checks, the pair pipeline, and linked addition with all thirteen packages loaded.
- The scalar tables are checked exhaustively against independent Rust oracles, including rejected targets, here and in the `arithmetic` and `language` suites.

Linked arithmetic is verified along direct execution paths by the package checks in `//library/natural` and the programs in `//program/ternary`.

## Extending

Add an operation as its own file in the package that owns its type, give it a `photonic_library` target with explicit dependencies and visibility, list it in the package's `source` filegroup, and add it to [`test/catalog.rs`](test/catalog.rs) so the isolation and composition checks cover it. A new type gets its own package and namespace. Callbacks register themselves with a dispatch rule such as `[Boolean.Not.([Each] Boolean.Not)] Function.Boolean.Not`, and a new chain alphabet declares its `Drop`, Reverse, and Erase rules without editing the chain package.

## Limits

Collections are finite pairs with enumerated payloads, and `Field` covers positions 0 through 3 with values 0 through 2. Photonic has no variables, so each alphabet enumerates its symbols. Linked values cannot enter an `Invoke` scope, because joins require one frame. Unbounded collections, general repetition, and parallel prefix networks remain open work.
