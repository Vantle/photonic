# Multiplication from two runtime numerals

[product.wave](product.wave) is a fixed ordinary-rule construction over two Unit numerals. The supplied instance is two times two. The rules contain no coefficient-specific Unit output: each copying action emits one fresh Unit, and the number of actions comes from the input data. There is no arithmetic primitive, capture syntax, negative premise, or numeral-specific matcher.

This is a retained-input reachability protocol. Its accepted result contains two archived operands and a Product coherence independent of their resources. It is not the bare expression `Multiply(Unit.Unit,Unit.Unit)` evaluated into a lone numeral. The helper rule values and complete target below are part of the interface. The small cases have closed positive and negative checks; the two-times-two target has a concrete witness, with exploration unfinished. A universal object-language correctness certificate remains unimplemented.

## Run the example

From the repository root:

```sh
bazel run -c opt //system:command -- obsidian \
  "$PWD/mathematics/natural/product.wave" \
  --target "$PWD/mathematics/natural/archive.particle" \
  --steps 15000000 --states 100000 --records 6000000 \
  --cells 128 --frames 128 --coherences 16
```

Recorded result:

```text
Reached: exact target configuration under the supplied program
Witness s45095: [Archive.Unit.Unit]@root [Archive.Unit.Unit]@root [Unit.Unit.Unit.Unit.Product]@root
45096 configurations; 480987 queued, 0 deferred; exploration unfinished
```

A reached target is positive evidence even while exploration is unfinished. This run does not prove that every other numerical target is unreachable. The search is expensive for such a small calculation; this is a semantic construction, not efficient arithmetic.

For addition, the existing simpler protocol remains:

```text
Add(Unit.Unit.Unit, Unit.Unit.Unit.Unit.Unit.Unit.Unit)
[Add, Add] ()
```

```sh
bazel run -c opt //system:command -- obsidian \
  "$PWD/example/group/addition.wave" --target "$PWD/example/group/ten.particle"
```

## Input and result

The first two coherences in product.wave each contain one fixed preparation rule value, a Multiply label, and the supplied Unit occurrences. Change only those two initial Unit counts to supply other operands; leave every rule body unchanged. One helper prepares the factor role and the other prepares the count role. Both input numerals use Unit. For zero, omit the Unit occurrences while retaining the helper and Multiply.

The accepted target for input counts m and n is:

```text
Archive.(m Unit occurrences),
Archive.(n Unit occurrences),
Product.(m × n Unit occurrences)
```

This display describes the protocol; the words and multiplication sign are not Photonic syntax. The actual two-times-two target is:

```text
Archive.Unit.Unit,
Archive.Unit.Unit,
Product.Unit.Unit.Unit.Unit
```

Writing the target as separate initial coherences gives its product occurrences independent introductions. Obsidian compares full canonical states, including sharing, rather than merely counting identical printed labels. Its existing checker supplies this distinction; there is no custom arithmetic target checker.

## How the rules work

1. Each preparation rule splits its input into an Archive coherence and a working coherence. Their inherited input Units share resource identities.
2. Scoped ordinary rules convert the working Units into X or Y. The distinct internal labels preserve the two operand roles after reunion. A premature return can leave unconverted Units; the language does not test for absence.
3. The main joint rule consumes both preparation rule values as ordinary operands, alongside Factor and Count. This removes the executable preparation capability from its body. Inferred applications at an earlier source cannot keep restarting preparation there through those consumed rule values.
4. A phase consumes a Y and splits into a continuing phase and a copying scope. The copying scope receives the inherited X numeral and converts each X into a fresh Unit. It also removes its inherited Y bookkeeping. Separate scopes give independently computed copies independent introductions.
5. The base cleanup removes the continuing X numeral. Done coherences reunite completed contributions, and the outer return supplies Product. A leaked X, Y, or control label prevents an exact clean numerical target.
6. Two ordinary meta rules remove the known preparation rule values from the Archive coherences. The archived original Unit occurrences remain.

The whole helper rules appear literally in the initial data and in the consuming patterns. Their repetition is visible in the source; no hidden alias expander or structural variable is involved. The runtime uses the existing whole-rule value matcher and lexical capture contract.

## Why retain the inputs?

The earlier preparation attempt could return unconverted Unit occurrences as if they were products. For one times one, that produced an apparent two-Unit result.

Here an unconverted Unit still shares its resource identity with an archived input. It cannot satisfy the target's independent Product occurrences. A fresh Unit comes from an explicit X-to-Unit action. Retaining the original inputs therefore distinguishes copying from simply forwarding an input occurrence, using the same resource identity that already distinguishes broadcast from independent introduction.

Archive and Product are ordinary atoms, not protected types. A Product label alone is not a certificate. Do not erase the archives before checking and then claim the same specification: that would discard the resource-sharing evidence. Arbitrary additional rules can change which targets are reachable, as with every Obsidian claim.

## Correctness argument and remaining proof obligation

A direct successful schedule first converts all working input units, then performs one complete copying scope per Y, then returns the reunited contributions. Each copying scope converts all m X occurrences into m independent Units. There are n such scopes, giving m × n fresh product units. This describes a finite successful schedule for finite operands, assuming sufficient execution resources.

For soundness, incomplete preparation must leave archived sharing; incomplete copying or cleanup must leave bookkeeping; and source-inferred shortcuts must not remove those obligations while dropping numerical contributions. The scope boundaries and consumed preparation code are intended to establish these invariants. The regressions below exercise them, but a complete proof covering every inferred application has not been mechanized. Do not upgrade finite tests to a universal soundness claim.

The full two-times-two example reaches its target. A two-times-three probe, changing only the second initial Unit count and its exact target, returned Unknown after a budget of 100 million work steps, 500,000 states, 12 million records, 128 cells, 128 frames, and 16 coherences with four workers. It recorded 96,180 configurations and 1,607,169 queued tasks, with no deferred applications. Its exploration was unfinished, so this is not evidence of unreachability. Broad validation and search efficiency remain open work before treating this protocol as a mature general arithmetic interface.

## Compose with addition

Supply an additional `Add.Unit` coherence and the ordinary rule:

```text
[Product, Add] Product
```

It consumes the two control labels and reunites their numerical remainders, retaining Product on the sum. For one times one followed by adding one, Obsidian reaches two fresh Product units beside the original archives. The Rust composition regression checks that exact target.

## Executable evidence

`system/test/product.rs` uses the same fixed source program and replaces only its initial Unit counts. For both input counts from zero through one, it checks result counts zero, one, and two with closed exploration. Exactly the mathematical product is reached. The test also checks multiplication followed by addition. The larger two-times-two command above is a separately recorded positive witness.

```sh
bazel test //...
```

The runtime now gives pending matching and application work a bounded head start over transitive view composition. Both queues remain FIFO; at most 4096 foreground removals occur before a pending composition is serviced. No inference is discarded or given a language-level priority. Queue fairness, budget resumption, chunking, worker-count determinism, and existing semantic reference tests cover the scheduling change.

## Larger numbers

1500 × 123 is 184,500. A unary product would require 184,500 Unit occurrences before accounting for archived inputs, intermediate states, or derivation evidence. The current prototype is not practical at that size. The unresolved two-times-three search shows that proof exploration is the immediate bottleneck even before representation size dominates. Compact numeral encodings and a more effective general proof-search strategy are necessary engineering work; increasing limits alone is not an adequate solution.

The [binary library](../binary/README.md) now demonstrates that alternative: ordinary fixed-width circuit rules and a generic direct-path proof strategy reach 1500 × 123 = 184500. This uses compact numeral data rather than expanding the unary protocol described here.
