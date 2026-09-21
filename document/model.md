# Structural construction reference

The independent Rust model in [model/](../model/model.rs) begins the executable semantic work required by [dynamic.md](dynamic.md) and the [recursive value contract](collection.md). It has no production frontend, matching, storage or proof-engine dependency. Build and run it through the pinned Bazel toolchain:

```sh
bazel test -c opt //model:test
```

## Structural boundary

The model distinguishes Value, Particle, Input, Destination, Output, Body and Rule. Particles, inputs, outputs and bodies canonicalize their unordered contents while retaining multiplicity. An input containing one empty particle differs from an empty input. An empty output differs from one empty destination, and an absent body differs from a present empty body. Types prevent an input configuration from being substituted directly into a value position.

A contextual Rule names its lexical context. Nested rule values and bodies retain their own context references. These references are nominal identities supplied by the model fixture; equal syntax in two distinct contexts remains different code. The model does not yet evaluate lexical declarations or canonicalize context graphs under administrative renaming.

Fragment carries immutable structural content and evidence. Its constructor is private to the model. Inspection of a concrete occurrence adds its read identity and reachable contextual dependencies. The construction API creates typed composites from fragments, unions their dependencies and preserves nested captures. Repeated code description insertion may duplicate syntax positions but retains the same read identity; it does not produce or transfer an additional live resource.

Opening a rule returns its input and output fragments plus a private origin. Closing that inspection reconstructs the original lexical context. Even replacing both visible components retains the origin's read and history dependencies. Constructing a new outer rule instead uses the construction context. These are different operations, including when their output text would look the same.

## History assumption

The first model uses explicit finite branch assignments. A history maps each named fork to one alternative. A construction may use a source only when its current history includes every source assignment with the same alternative. A missing assignment is insufficient evidence, and a different assignment is a conflict. Extending an assignment is immutable and rejects attempts to replace an existing alternative.

This is an explicit restricted compatibility model, not an implementation of Photonic's full source inference or grounded proof support. The fixture supplies valid concrete occurrence identities and contexts; the model is not a validator for arbitrary external reports. Deriving assignments from production events, checking consuming conflicts, importing lexical environments and projecting inferred witnesses remain separate work.

Evidence records source reads, contextual dependencies and the current branch assignment. It intentionally retains conservative dependencies rather than minimizing them. Rule reconstruction must not erase a dependency just because a source component becomes empty or is replaced. Unknown atoms remain literals; there is no spelling convention that turns them into variables.

## Tested laws

The first suite checks arbitrary role/payload spelling, construction from inspected role and nested-rule payload occurrences, exact reconstruction through depth 16, distinct empty forms, permutation and multiplicity, mixed contextual insertion, body origins, output replacement and retention of a private origin when both components are replaced. A 729-pair history matrix independently checks assignment inclusion and inspection admission. Host-driven structural construction exercises depths 0, 1, 2, 3, 8, 16 and 32.

Host-driven generation demonstrates that the model representation can grow new structures. The staged model below additionally represents and evaluates generator factories. Neither establishes native structural execution, a runtime-recursive Photonic generator or a collection library. The construction operations remain atomic reference primitives; the machine below adds resumable template traversal. No universal preservation proof is claimed from finite tests.

## Binding and templates

Scope allocates typed slots with a nominal lexical owner and a unique position within that scope. Literal atoms never become references by convention. Binding stores immutable fragments under slots of one structural sort; immutable nested environments retain access to ancestor slots, reject attempts to bind an ancestor from a child, and reject reuse of an active scope identity. Rebinding an occupied slot is an error. A reference to a missing slot or inaccessible scope produces a structured failure.

Templates distinguish literal construction from references at each supported sort: value, particle, input, output and body. Building a rule or body uses the current construction context. Substituting a captured rule or body retains its original context and evidence. Every reference is checked against the current construction history before insertion. The output type contains only closed structures, so an unresolved reference cannot be returned as executable code.

The template suite checks unknown roles and contextual payloads, administrative renaming of binder scope identities, ancestor access and child isolation, duplicate binding rejection, missing external references, all supported structural sorts, body capture retention and history rejection on substitution. Repeated substitution preserves syntax multiplicity without manufacturing distinct source reads. One fixed template wraps increasing depth through 32 host-driven instantiations.

## Generated bindings

The staged construction model represents a generator as a declaration of future typed parameters and a term. A term either builds a closed value or quotes another generator definition. Parameter declarations support all five binding sorts, reject duplicate positions and foreign owners, and preserve literal atom semantics. Quotation validates references through every nested definition: references owned by an enclosing future declaration may remain unresolved; every external reference must resolve in the captured environment and pass the current history check. A reference with an undeclared position is rejected even when its owner is a known future scope.

A closed generator captures an immutable environment and the evidence needed by its external references. Application supplies exactly the declared parameters with their exact sorts in a private nested environment. All argument and generator dependencies are retained, including for an argument the body does not use. A quoted child captures its parent's now-bound parameters while retaining its own unfilled future parameters. Separate invocations of one factory can therefore produce separate builders without exchanging bindings, histories or payload captures.

The factory test represents a generator whose role parameter becomes the input of a generated field builder. The returned builder owns a new payload parameter; applying it creates ordinary closed rule structure. Two invocations retain different roles, while a nested rule payload retains its own lexical context and source identity. Further tests cover binder renaming, missing external and undeclared local references, duplicate/foreign/mismatched declarations, all parameter sorts, arity, ancestor histories, nested capture evidence and inactive future evidence at quotation time.

Generator products belong to the reference construction machinery, not to live Photonic configurations. This is a staged semantic experiment with explicit quoted definitions, not production syntax or a second native callable value kind. Scope identities in fixtures are nominal; invocation behavior is tested under their renaming, while canonical equality of quoted code remains unimplemented. Concrete rule matching, consumption, lexical declaration activation and source-inferred projection still need their own execution laws before this representation can be integrated into ordinary rule values.

## Resumable construction

Machine interprets a template through an explicit task stack, visiting one child at a time and retaining private partial composites. Each task transition consumes one model work unit. `run(0)` leaves unfinished construction unchanged. A successful result is published only after the complete outer value is assembled; a failure is terminal and discards partial work. Further calls after either outcome preserve the outcome and consume no additional work. Immutable borrowed templates, environments and construction contexts prevent mutation beneath a suspended cursor.

The recursive template interpreter remains the independent traversal oracle. Machine tests compare both success and structured failure at every single suspension boundary for empty shapes, nested code, branching particles, bodies, references and missing or incompatible bindings. They also compare repeated budgets of 1, 2, 3, 7 and 32 work units and repeated observation after completion.

These work units specify reference-machine progress, not production runtime step accounting. Fragment cloning, dependency union and multiset sorting still occur within individual primitives. Consequently this is cooperative resumability between structural operations, not a wall-clock preemption or hard-memory guarantee. Native integration must separately account for those primitive costs, code/environment limits and production observation compatibility.

An invocation that produces a closed value uses the same machine and retains generator and argument support in its final result. Suspension-prefix tests compare this path with direct invocation, including unused-argument evidence. Quotation validation and parameter preparation remain atomic reference operations; the machine API explicitly rejects a quoted-generator term rather than pretending its validation has been budgeted. Resumable quotation is still required before production integration.

## Verification

The foundation, template, machine and staged generator increments pass all 32 model tests through `bazel test -c opt //model:test`, including the 729-pair history matrix inside one test and the complete suspension-prefix comparisons. The build runs Rustfmt, Clippy and Bazel checks. `bazel test -c opt //...` reports 108 passing targets: the expanded model suite ran freshly and 107 unchanged targets used valid cached results. This is local native build evidence, not a new cross-platform run, a benchmark, or native/WebAssembly structural conformance.

## Next work

Next add concrete application with contextual lookup and explicit read/consume ownership. Activation of a local declaration must distinguish its new execution frame from an embedded closure's existing capture; simply rebinding every nested context would break the mixed-capture contract. Establish that distinction and source-inference projection before selecting production syntax or claiming mixed-capture execution conformance. Resumable quotation, quoted-code canonical equality and runtime-recursive generation also remain required. Native and WebAssembly integration must compare against the model on the actual shared semantic domain.
