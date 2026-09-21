# Complete recursive values

This contract implements the first specification milestone of the [roadmap](roadmap.md). It is a specification for the native library and structural reference model, not accepted new source syntax. Production support and conformance remain queued.

## Carrier

A collection is a finite ordered binary tree. Its mathematical constructors are Empty, Leaf(value) and Branch(left, right). A value may be an atom or a complete contextual rule. Empty is an explicit closed shape; silence, an absent branch and a pending computation are not Empty. Branch has exactly two children, including any explicitly empty child. Association and child position are structural, not inferred from particle order or flattened labels.

The tree is immutable. Internal storage sharing does not introduce a second live resource. Each leaf carries its payload occurrence identity and, for code, its capture and provenance. Two equal printed leaves may represent distinct occurrences. Two paths referencing one inherited occurrence retain that sharing. A consumer must not turn the latter into two independently consumable operands. The ground runtime permits a joint match to consume an introduction shared across distinct coherences; this is one shared resource, not two independent introductions. Within one particle, distinct matched positions still require distinct occurrences.

An open construction is distinct from this carrier. A missing child or unresolved external binding cannot be published as a complete Branch. Construction can advance independently in either child, but publication requires both complete children and compatible evidence. Empty input is accepted only through an explicit Empty value.

The initial implementation need not expose these constructor names as special language atoms. Choose an ordinary Photonic encoding after the structural model establishes the boundary laws. The existing pair truth tables and linked numeral protocol are useful comparisons, but neither establishes generic unknown-payload construction. Do not add collection primitives to the evaluator.

## Operation boundary

A request associates one supplied operation with one complete input and a fresh invocation identity. An operation receives a whole payload descriptor with its context, rather than an untagged remainder. A result associates a whole output payload with the same invocation and compatible execution evidence. Forwarded code retains its captures. Code used as the operation contributes executable-read evidence independently of any argument consumption.

Map preserves Empty and Branch structure. For each Leaf, it invokes the supplied operation in an independent local scope and replaces that leaf only after a complete result arrives. Results from different invocations cannot satisfy each other's completion. Map is complete only when the resulting tree and all of its leaf evidence are complete. A supplied operation that never completes leaves its parent incomplete. An operation with competing results can produce separate supported result trees; its alternatives must not be combined as jointly available evidence.

Gather reconstructs Branch from two complete children and their association evidence. Its result preserves each child's payload, capture, occurrence sharing and history dependencies. It cannot match only Done or Return and recover payloads by assumption. The existing marker-only counterexample remains an accepted counterexample, not a program whose scheduling should be changed.

Reduce takes an explicit empty identity and a supplied binary operation. Empty returns the specified identity; Leaf returns its payload according to that payload's transfer contract; Branch combines the complete reductions of left and right in that order. This defines a tree reduction for arbitrary supplied operations. Reassociation, balancing and parallel prefix require separately established associativity and identity laws. Noncommutative operations retain left/right order. Fold and Loop are separate protocols with their own causal dependencies.

## Ownership and construction

| Action | Payload identity | Evidence |
| --- | --- | --- |
| Inspect a rule component | No live component occurrence is created | Read dependency on the complete source occurrence and its contextual origins |
| Embed inspected code in a newly constructed rule | New outer occurrence; embedded closures retain captures | Construction plus code-read and context dependencies; no fresh captured resource |
| Forward a whole payload | Preserve the existing occurrence identity | Existing transfer flow and compatible source evidence |
| Produce a literal value | Fresh occurrence | Production event and its required support |
| Gather complete children | Preserve the declared child transfer/reconstruction contract | Both child payloads, invocation association and compatible histories |

Code description insertion and live occurrence transfer are different operations. A repeated syntax reference can share immutable structure; it does not authorize consuming one occurrence twice. A newly constructed wrapper belongs to its construction context. An embedded existing closure retains its original context. Reconstructing an inspected rule unchanged must preserve the original contextual structure, rather than rebinding it in the inspector's context.

Source inference must commute with this boundary: projecting a supported construction or operation result must account for the same payload and contextual dependencies as the corresponding concrete witness. Until the structural projection law is implemented and checked, structural inspection accepts concrete closed witnesses only. This restriction belongs to the staged reference model; it must not silently change existing ground inference or be reported as full production support.

## Required evidence

| Case | Required observation |
| --- | --- |
| Empty input | Map returns explicit Empty; Reduce returns the supplied identity |
| Increasing runtime size | One fixed program handles several tree depths without pre-enumerating their complete shapes |
| Unknown payload | Unseen atoms and nested contextual rules survive identity mapping and gather |
| Directional operation | Swapping Branch children changes a noncommutative result; textual particle permutations do not |
| Missing child | No complete parent can be produced |
| Marker without payload | No complete parent or reduced result can be produced |
| Shared occurrence | Two references remain shared; a consuming binary operation cannot use them as independent resources |
| Equal fresh occurrences | Equal values with independent introductions remain independently consumable |
| Mixed captures | Equal code from different contexts retains its distinct lexical behavior after reconstruction |
| Competing evidence | A parent cannot gather incompatible alternatives |
| Repeated invocation | Completion and payload cannot cross invocation boundaries |
| Suspension and pressure | Resumption and eviction agree with uninterrupted execution; limits yield unfinished work |

The reference model first establishes exact structural sorts, contextual reconstruction and evidence composition. Native library execution then checks the contract against concrete and inferred runtime behavior. Small cases need positive and forbidden complete configurations under exhaustive exploration; larger cases need direct witnesses plus bounded differential and metamorphic checks. A direct witness alone does not establish the forbidden outcomes. Native and WebAssembly must use the same production semantics, compared with the independent model on their common domain.

## Queue

The [independent model](model.md) now implements typed contextual fragments, construction, scoped binding, templates, resumable traversal, staged generator factories, concrete application with fresh local declaration binding, checked introduction from live read witnesses, flow composition, replayed paths, projected bindings and restricted branch-history compatibility. Thirty-eight native comparison cases cover restricted ground behavior and projection evidence. Its construction-language factories and host-driven growth tests do not establish native Photonic generation.

1. Apply projected bindings to source configurations with correct scoped consumption and complete captured-frame import, then extend introduction beyond live single-world read witnesses.
2. Preserve auxiliary read support across historical and inferred events, and complete resumable quotation and quoted-code equality before native integration.
3. Select source syntax and implement generic construction, then the collection library and runtime-sized demonstration.

Completing this document closes the specification deliverable only. It does not establish the laws experimentally, provide a native collection implementation or settle the source-inference proof obligation.
