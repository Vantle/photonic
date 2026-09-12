# Structural values

Rust extends the accepted core with one structural matching and construction mechanism for data and code. The JavaScript kernel remains an independent reference for the earlier ground fragment. The [interactive plan](plan.html#structure) includes captured Rust execution reports for this extension.

## Values and patterns

A value is an atom, a named structure containing an unordered particle, or a complete rule with its lexical capture. A variable such as `$value` captures one complete value. A constructor pattern inspects the explicit structure of that value. Literal matching is the zero-variable case of the same operation.

```text
Box(A.B);
[Box($left.$right)] -> Pair(Left($left).Right($right));
```

Both assignments are possible because particles are orderless. The results are `Pair(Left(A).Right(B))` and `Pair(Left(B).Right(A))`. Use named fields such as `Left(...)` and `Right(...)` when roles must be distinguished. `Pair(A.A)` retains two members; it is not `Pair(A)`.

Repeated variables require equal complete values, including captured scopes. Their operand positions still require distinct occurrences. `[$x.$x]` cannot consume one occurrence twice. `[Left($x), Right($x)]` requires distinct compatible coherences carrying the same bound value. Each completed binding reaches ordinary source projection and application; there is no macro evaluator, type evaluator, or compile-time execution phase.

## Code construction

```text
Box(@([A] -> Old)).A;
[Box(@([$input] -> Old))] -> @([$input] -> New);
```

The rule captures `A`, consumes the containing `Box(...)` occurrence, and constructs `@([A] -> New)`. Exposing that rule as a live value makes it eligible through ordinary local activation. The boxed original is inactive. Invoking the constructed rule retains its read dependence; explicitly matching a rule value consumes its containing occurrence.

A constructor pattern inspects the fields written in the pattern. An implicit closure capture is not an additional lexical-equality condition on that pattern. This applies uniformly, whether or not the pattern contains variables. Complete values bound to repeated variables do compare their captures. This deliberately replaces the earlier ground reference's capture-constrained whole-rule pattern lookup; capture-aware value and configuration identity remain intact.

Inspecting a nested field does not split code into independent live resources. Substitution carries a value; the containing occurrence supplies the consuming footprint. When a derived view enables a match, ordinary projection applies the result at the concrete source. Intermediate evidence is not appended to the result.

## Scope and identity

Captured fragments retain their original closures. Newly constructed code captures the transformer's defining frame. An enclosing constructed rule and an inserted rule fragment can therefore refer to different frames. Invocation still supplies a separate return continuation.

Runtime values contain these capture edges explicitly, including captures buried inside structures, rule fields, and body declarations. Reachability, import, state canonicalization, and report serialization traverse them. Import uses one shared frame and occurrence mapping for the entire application, preserving shared versus independent environments.

Instantiated rule content is structural; a template identifier and substitutions are not semantic identity. Constructing `@([A] -> B)` by substituting `B` agrees with writing that rule directly under the same capture. Diagnostic names and allocation addresses do not affect equality. Local binder renaming is normalized, while nested rule constructors retain lexical references to enclosing bindings until those bindings are substituted.

A variable occurring only in an absence premise is existential within that query. A variable already bound by the positive input constrains the query to that value. Query constraints preserve captured sharing and frame anchors through witness projection. A free output variable has no concrete value to construct, so that application has no successor; the program is not rejected as logically invalid. Unresolved variable expressions in an initial configuration likewise do not become arbitrary concrete values.

## Deliberate boundary

A variable captures one complete runtime value, not an arbitrary particle remainder or a binder declaration. Whole rules may contain local binders and can be captured and moved intact. A local variable slot inside inspected code is pattern syntax, not an independent runtime value to extract. Editing binder syntax would require an explicit representation of binder ownership; silently treating a variable's spelling as that ownership would be incorrect.

The present constructors have fixed, explicit shape. They can inspect nested data and rule fields, preserve unknown complete value fragments, and construct new executable rules. Particle-rest variables, arbitrary binder-syntax editing, and behavioral equivalence are not implemented. These boundaries do not impose a fixed universe of producible data or rule structures.

## Execution bounds

Structural assignment is a resumable search on the existing agenda. Equal candidates and compatible gate prefixes are reused; a result binds values and retains the containing occurrence identities. Independent worker steps merge deterministically.

Exact identity includes multiplicity, sharing, nested captures, and continuations. State reuse suppresses repeated equal configurations; it does not terminate a program that continually constructs larger values. Escaped closures can also retain increasingly deep parent/held history even when their displayed code repeats. Those are distinct capture graphs under the existing contract; discarding that history would require a separate semantic justification. Retained-record accounting includes stored value structure and suspends oversized successors. It is a logical accounting limit, not a byte cap.

Canonicalization and alpha normalization have difficult symmetry cases. Binder normalization removes provably interchangeable assignments and refines distinguishable occurrences, but exact residual enumeration can still be factorial. Compilation, closure normalization, constraint comparison, and reporting are not strict preemption points. The implementation does not claim hard real-time or unrestricted scalability guarantees.
