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

Host-driven generation demonstrates that the model representation can grow new structures. It does not establish an object-language generator, generated generators, native structural execution or a collection library. The present operations are atomic reference functions; they do not yet establish resumable construction or hard memory bounds. No universal preservation proof is claimed from finite tests.

## Binding and templates

Scope allocates typed slots with a nominal lexical owner and a unique position within that scope. Literal atoms never become references by convention. Binding stores immutable fragments under slots of one structural sort; immutable nested environments retain access to ancestor slots, reject attempts to bind an ancestor from a child, and reject reuse of an active scope identity. Rebinding an occupied slot is an error. A reference to a missing slot or inaccessible scope produces a structured failure.

Templates distinguish literal construction from references at each supported sort: value, particle, input, output and body. Building a rule or body uses the current construction context. Substituting a captured rule or body retains its original context and evidence. Every reference is checked against the current construction history before insertion. The output type contains only closed structures, so an unresolved reference cannot be returned as executable code.

The template suite checks unknown roles and contextual payloads, administrative renaming of binder scope identities, ancestor access and child isolation, duplicate binding rejection, missing external references, all supported structural sorts, body capture retention and history rejection on substitution. Repeated substitution preserves syntax multiplicity without manufacturing distinct source reads. One fixed template wraps increasing depth through 32 host-driven instantiations. Generated object-language binding forms and resumable template evaluation remain unfinished.

## Verification

The foundation and template increments pass all 19 model tests through `bazel test -c opt //model:test`, including the 729-pair history matrix inside one test. The build runs Rustfmt, Clippy and Bazel checks. `bazel test -c opt //...` reports all 108 repository test targets passing using valid cached results, including the newly executed model suite. This is local native build evidence; it is not a new cross-platform run, a benchmark, or native/WebAssembly structural conformance.

## Next work

Add a small resumable construction machine and generated binder forms without allowing external unresolved bindings to become executable. Extend the reference with concrete application and source-inference projection before selecting production syntax or claiming mixed-capture execution conformance. Native and WebAssembly integration must compare against the model on the actual shared semantic domain.
