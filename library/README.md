# Photonic standard library

The standard library is written entirely in Photonic. Rust loads the sources and checks their behavior; every operation runs through ordinary rules. Fourteen packages cover calling conventions, scalar tables, finite collections, linked storage, unbounded arithmetic, ordered vectors, and expression evaluation. Any combination of them can be loaded together.

```sh
bazel test -c opt //library/...
```

## Principles

1. **One namespace per type.** Every operation is named `<Type>.<Verb>`: `Boolean.Not`, `Ternary.Add`, `Natural.Divide`, `Expression.Evaluate`. The namespace belongs to exactly one package. Only the core combinators `Identity` and `Compose` are bare.
2. **One calling vocabulary.** A request carries `Function`; its answer carries `Return`. Scoped calls use `Invoke`; linked calls tag each answer with the operation that produced it.
3. **Collision-free by construction.** `//library:test` proves that no root rule of one package can match the input of another package's rule, then runs checks with every package loaded at once.
4. **Explicit values.** Roles travel as fields such as `([Digit] 2).([Carry] 1)`. Alternatives are variants of the answer, and failures are `Error.<Kind>`.
5. **Generic storage, declared alphabets.** A chain stores any symbol, including a vector. Each alphabet states how chains drop, reverse, and erase its symbols. A vector stores linked values, such as naturals, by reference.
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
| [natural](natural/) | `Natural` | `digit`, `copy`, `trim`, `normalize`, `successor`, `complement`, `column`, `add`, `borrow`, `subtract`, `difference`, `multiply`, `divide`, `compare` |
| [integer](integer/) | `Integer` | `add`, `subtract`, `multiply`, `divide`, `result` |
| [expression](expression/) | `Expression` | `token`, `split`, `join`, `parse`, `execute`, `evaluate` |
| [vector](vector/) | `Vector` | `node`, `reverse`, `erase`, `merge`, `sort` |

Targets carry their dependencies, so `deps = ["//library/natural:divide"]` loads the chain, the digit alphabet, copying, subtraction, and normalization it needs. The scalar packages depend on `//library/function:invoke`. The layering is:

```
function ── boolean, ternary, binary, carry, field
         └─ collection ── selection
chain ── natural ── integer ── expression
     │         └── vector sort
     └── vector
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
| `Function.Natural.Compare` | `Operand.Left`, `Operand.Right` | `Return.Natural.Compare.Less`, `.Equal`, or `.Greater`, with `Return.Natural.Compare.Left` and `Return.Natural.Compare.Right` beside the unchanged operands |
| `Function.Integer.<Verb>` | `Operand.Left.<Sign>`, `Operand.Right.<Sign>` | `Return.Integer.<Verb>.Positive`, `Return.Integer.<Verb>.Negative`, `Return.Integer.Divide.Error.Divisor` |
| `Function.Expression.Evaluate` | beside the token tape | `Return.Expression.Evaluate.Positive`, `Return.Expression.Evaluate.Negative`, `Return.Expression.Evaluate.Error.Syntax`, `.Error.Stack`, `.Error.Divisor` |
| `Function.Vector.Reverse` | beside the vector | `Return.Vector.Reverse` |
| `Function.Vector.Erase` | beside the vector | `Return.Vector.Erase` |
| `Function.Vector.Sort` | beside a vector of naturals | `Return.Vector.Sort` |

Numerals store base-three digits least significant first. Arithmetic answers carry no leading zeros; integers carry `Positive` or `Negative`, zero is always `Positive`, and integer division truncates toward zero.

`Function.Natural.Compare` reads both operands without consuming them. It peeks one column at a time, least significant first; the most significant differing column decides, and a finished operand reads as zero, so high zeros compare equal to none. Each column costs about twenty events, and the verdict arrives only after both operands are released unchanged.

## Chains

A chain handle is `Zero` or a `Head` coherence carrying a private seal and its methods; its cells remain elsewhere in the same frame.

| Request beside a chain | Answer |
| --- | --- |
| `Push.<symbol>` | `Built` beside the extended chain |
| `Read` | `Yield.<symbol>` beside the tail, or `Yield.End.Zero` |
| `Forget` | `Clean` once this reference is dropped |
| `Peek` | `Seen.<symbol>` beside a reference to the tail, or `Seen.End.Zero`; the cells are unchanged |

A join hands its unmatched remainder to every output, so moving a chain means forgetting it everywhere else. The move idiom sends the chain to its next use and resumes once the old reference is clean:

```
Push.([Digit] 1).Zero, Stage.1
[Built, Stage.1] (Push.([Digit] 2)) (Forget.Stage.2)
[Built, Clean.Stage.2] (Operand.Left) (Forget.Stage.3)
```

`Peek` consumes the reference it is given, so peek through an alias and keep the original. Each cell rebuilds itself from the remainder of the peek, so the `Peek` coherence must hold nothing but the request and the reference. Walking a numeral by `Peek` costs four events per cell and leaves every cell in place.

`Push` is symbol-agnostic. An alphabet declares one `Drop` rule per symbol so a new head can forget its predecessor, plus the hooks that let `Reverse` and `Erase` traverse its symbols. The digit alphabet reads:

```
[Drop.([Digit] 0)] Forget
[Yield.([Digit] 0).Reverse.Wait] (Reverse.Left) (Forget.Reverse.Write.([Digit] 0))
[Clean.Reverse.Write.([Digit] 0), Reverse.Right] (Push.([Digit] 0)) (Forget.Reverse.Await)
[Yield.([Digit] 0).Erase.Wait] Read.Erase.Wait
```

`//library/natural:digit` declares the digits and `//library/expression:token` declares `Mark`, `Literal`, `Add`, `Subtract`, `Multiply`, `Divide`, `Open`, `Close`, `Negative`, and `Negate`. The `//library/chain:cell.check` test stores a caller-defined symbol with nothing but its own `[Drop.Item] Forget`.

## Vectors

A vector handle is `Empty` or a `Node` coherence carrying a private seal and its methods. Each node's `Slot` holds one item and the rest of the vector. Items are linked values that answer the chain's `Forget`, such as naturals, and a vector holds only their handles: inserting, taking, sorting, and reversing move references, never digits.

| Request | Answer |
| --- | --- |
| `Insert` beside an item and a vector | `Stored` beside the extended vector |
| `Take` beside a vector | `Taken.Item` beside the front item and the rest, or `Taken.End.Empty` |
| `Release` beside a vector | `Released` once this reference is dropped |
| `Drop` beside a vector | `Forget`, so a chain can store vectors as symbols |

Like `Push`, `Insert` builds its slot from the remainder of its request, so that coherence must hold nothing but `Insert`, the item, and the vector. `Take` carries any other atoms into its answer. Split the item from the rest by forgetting one and releasing the other:

```
[Built, Stage.1] Insert.Empty
[Stored, Stage.2] Take.Next
[Taken.Item.Next] (Forget.Rest) (Release.Value)
```

`Clean.Rest` then holds the rest and `Released.Value` holds the item. `Function.Vector.Erase` erases every item with `Function.Chain.Erase`, so their alphabets must declare erase hooks.

### Sorting

`Function.Vector.Sort` orders a vector of naturals ascending with a stable, adaptive merge sort:

1. **Runs.** One pass splits the input into maximal non-decreasing runs and strictly decreasing runs, comparing each neighbouring pair once. Strictness keeps equal items in input order when a decreasing run is reversed.
2. **Balanced passes.** Runs wait on a chain of vectors. Each pass merges them pairwise into a second chain, so `r` runs take `⌈log₂ r⌉` passes. An odd run is carried into the next pass unmerged.
3. **Alternating orientation.** A merge pushes onto the front of its output, which reverses its order. Passes therefore alternate between merging descending runs by taking the larger front and ascending runs by taking the smaller, and no merge result is ever reversed. Only a carried run is turned before its next merge, and the result is turned once if it ends descending.

A tie takes the item from the earlier run, which makes the sort stable. Sorted and strictly decreasing inputs form a single run and finish after `n − 1` comparisons with no merge pass. Otherwise the sort makes at most `n − 1 + n⌈log₂ r⌉` comparisons, close to the `log₂ n!` bound on random input, and `O(n log r)` reference moves. Comparisons dominate the cost; beyond its comparison, a merge step moves one item for about thirty-five events.

Powersort and Timsort choose each merge from run lengths or positions, which also balances runs of very different lengths. Tracking lengths here needs natural arithmetic at every run boundary, so the passes balance merges by run count instead, which is optimal when runs have similar lengths.

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
- `composition` runs scalar checks, the pair pipeline, linked addition, and a sort with all fourteen packages loaded.
- `natural` compares every pair below 27, operands with high zeros, and wide random pairs against Rust's ordering, reading both operands back afterwards.
- `vector` sorts every permutation of four items, repeated items, sorted, decreasing, and constant inputs, and seeded random vectors of up to twelve items against Rust's stable sort. Equal numerals with different high zeros check stability. It also reverses and erases vectors.
- The scalar tables are checked exhaustively against independent Rust oracles, including rejected targets, here and in the `arithmetic` and `language` suites.

Linked arithmetic, comparison, and vectors are verified along direct execution paths by these generated checks, the package checks in `//library/natural` and `//library/vector`, and the programs in `//program/ternary` and `//program/vector`.

## Extending

Add an operation as its own file in the package that owns its type, give it a `photonic_library` target with explicit dependencies and visibility, list it in the package's `source` filegroup, and add it to [`test/catalog.rs`](test/catalog.rs) so the isolation and composition checks cover it. A new type gets its own package and namespace. Callbacks register themselves with a dispatch rule such as `[Boolean.Not.([Each] Boolean.Not)] Function.Boolean.Not`, and a new chain alphabet declares its `Drop`, Reverse, and Erase rules without editing the chain package.

## Limits

Collections are finite pairs with enumerated payloads, and `Field` covers positions 0 through 3 with values 0 through 2. Photonic has no variables, so each alphabet enumerates its symbols. Linked values cannot enter an `Invoke` scope, because joins require one frame. For the same reason, linked calls in one frame run one at a time: `Insert`, `Function.Vector.Sort`, and `Function.Natural.Compare` use fixed labels and must not overlap. A vector cannot hold vectors, because a take could not tell an item from the rest; a chain can hold vectors. `Function.Vector.Sort` orders naturals only. General repetition and parallel prefix networks remain open work.
