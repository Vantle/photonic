# Documentation

The [Photonic webbook](../index.html) is the primary reading path: philosophy, grammar, coherences, abstraction, metaprogramming, runtime execution, natural numbers, ternary arithmetic, and runnable proofs. It opens directly from the filesystem and uses only local assets.

## Book assets

- [book.css](book.css): responsive reading layout, light and dark appearances, and print styling.
- [book.js](book.js): navigation, explanatory controls, and recorded-event navigation.
- [native.js](native.js): native Rust configuration and event reports for the language examples.
- [record.js](record.js): compact direct-path event records from the checked-in ternary arithmetic programs and streaming successor.

The book distinguishes browser illustrations from recorded Rust execution. Arithmetic illustrations use JavaScript to explain numerical representations; recorded-event controls navigate native results. The standalone source programs and CLI commands reproduce those results.

## Implementation contracts

| Subject | Reference |
| --- | --- |
| Grammar and executable lowering | [syntax.md](syntax.md) |
| Joint configurations, projection, and application | [semantics.md](semantics.md), [binding.md](binding.md) |
| Core runtime organization | [runtime.md](runtime.md) |
| Vocabulary | [terminology.md](terminology.md) |
| Native performance measurements | [performance.md](performance.md), [symmetry.md](symmetry.md) |
| Natural-number carrier and addition | [definition](../mathematics/natural/definition.md), [laws](../mathematics/natural/law.md) |
| Arithmetic interface and algorithms | [arithmetic](../mathematics/arithmetic/README.md) |

The [reference laboratory](plan.html) retains the independent JavaScript evaluator in `kernel/`. Native conformance fixtures also remain available to the Rust tests. These detailed contracts and executable fixtures support the single-page textbook.
