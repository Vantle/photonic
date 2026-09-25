# Program symmetry

An atom is a name and nothing more: rules compare atoms only for equality. Renaming the atoms of a program, one to one, renames every configuration it reaches and every event between them. What a program means is therefore the pattern its names form, its shape, and programs that differ only by their names are one program, whatever field their names come from.

The [symmetry](../symmetry/) package decides this. For any program, together with a target or with atoms whose names must stay, it computes a canonical form shared by every renaming, the renaming between two programs when one exists, and the automorphism group: every renaming that leaves the program unchanged. `photonic compare`, `photonic symmetry` and `photonic form` expose it, and the [Connections](../index.html#connection) chapter of the webbook runs it in the browser.

## Structure as a graph

A program becomes a vertex-colored directed graph whose atoms are the only unlabeled vertices.

| Vertex | Color | Edges to |
| --- | --- | --- |
| atom | one color for every free atom; a distinct color for each atom named by `--fix` | nothing |
| particle | particle | its atoms and rule values, one edge per distinct value, labeled with its multiplicity |
| rule | rule | its input particles and its outputs |
| output | output | its particle and the rules of its scope |
| part | the part's role: program or target | its live rules and initial coherences |

Every edge carries its kind and a count. Identical subtrees share one vertex: a particle that holds `([Of] Zero)` twice has one edge of count 2 to one rule vertex, and every occurrence of `([Of] Zero)` in the program is that vertex. Two distinct structure vertices therefore always differ as subtrees, so once every atom has its own color, refinement gives every structure vertex its own color too. The search only ever individualizes atoms.

## Canonical labeling

The algorithm is individualization and refinement, the method of nauty, bliss and Traces.

**Refinement** splits cells of an ordered partition until it is equitable: vertices in one cell have the same number of edges of each kind, counted with multiplicity, into every cell. A queue of splitter cells processes each edge kind in both directions; a split cell queues all its fragments except the largest, unless it was queued already. Fragments are ordered by their count and every split is folded into a trace, so the refinement and its trace depend only on the structure, never on vertex numbers.

**Search.** When refinement leaves atoms indistinguishable, the search picks the smallest cell of atoms and tries each member as a fixed point, refining again after each choice. A leaf is a discrete partition. Its certificate lists every vertex in the leaf's order with its color, its number of edges and each edge's target position, kind and count, so equal certificates mean equal relabeled graphs. The canonical labeling is the leaf that is least first by its traces, level by level, then by its certificate.

**Automorphisms.** Two leaves with equal traces and certificates differ by an automorphism, which the search records and uses in three ways.

- Children of a node that lie in one orbit of the automorphisms fixing the node's path lead to isomorphic subtrees; only one is explored.
- After finding an automorphism, the search returns to the level where the new path left the known one, because the subtree it was exploring is an image of one already explored.
- A node whose trace differs from the first path's and exceeds the best path's is pruned: it can hold neither the canonical leaf nor an image of the first leaf. The comparison with the best path restarts at equal whenever the best leaf changes, because a new best leaf always lies below the node being explored.

Along the first path, the size of each orbit of the first path's next vertex, under the automorphisms fixing the path so far, is the index of one stabilizer in the next, so their product is the order of the group.

**Blocks.** Before the search, atoms with identical occurrences everywhere, the same particles with the same multiplicities, are contracted into one vertex colored by their number. Such atoms always appear together, so any permutation of a block is an automorphism: `Boolean` and `And` in `library/boolean/and.particle` form one block. The search runs on the contracted graph, and each block of k atoms multiplies the group's order by k!. Reports separate blocks from the symmetries that move whole blocks.

The canonical order of the atoms, blocks expanded, renames the program into its shape, itself a program: two programs are isomorphic exactly when their shapes are equal. Comparing several programs groups them into classes of one shape. Each class carries a dictionary, one row per atom of the shape naming it in every member, the shape itself, and its symmetries. Within a block the dictionary keeps an atom's name wherever it can.

## Commands

```sh
bazel run -c opt //command:photonic -- compare "$PWD/library/boolean/and.particle" "$PWD/library/boolean/or.particle"
bazel run -c opt //command:photonic -- symmetry "$PWD/library/ternary/compare.particle"
bazel run -c opt //command:photonic -- form "$PWD/library/boolean/and.particle"
```

`compare` groups its programs by shape, prints the renaming from the first of each class to the others, and fails when there is more than one class. With `--json` it prints each class's members, its dictionary and its number of symmetries.

```
library/boolean/and.particle
  = library/boolean/or.particle by And → Or (True False)
```

`symmetry` prints the order of the automorphism group, the blocks, every symmetry when the group has at most 64 elements and generators otherwise, and each orbit of rules that the symmetries exchange:

```
12 atoms, 9 rules, 24 automorphisms
Block Compare.Function.Ternary
Symmetry (Left Right)(0 2)
Symmetry (Left Right)(Less Greater)
Symmetry (0 2)(Less Greater)
Orbit
    [Function.Ternary.Compare.([Left] 0).([Right] 0)] Return.Equal
    [Function.Ternary.Compare.([Left] 2).([Right] 2)] Return.Equal
Orbit
    [Function.Ternary.Compare.([Left] 0).([Right] 1)] Return.Less
    [Function.Ternary.Compare.([Left] 1).([Right] 0)] Return.Greater
    [Function.Ternary.Compare.([Left] 1).([Right] 2)] Return.Less
    [Function.Ternary.Compare.([Left] 2).([Right] 1)] Return.Greater
Orbit
    [Function.Ternary.Compare.([Left] 0).([Right] 2)] Return.Less
    [Function.Ternary.Compare.([Left] 2).([Right] 0)] Return.Greater
```

Comparing y with x answers the opposite of comparing x with y, and reflecting the trits through 1 reverses their order: each symmetry is a law of the table that holds for every input at once. The table's nine rules fall into four orbits, and the rule comparing 1 with 1 forms an orbit of its own. The three atoms of the request always travel together, which accounts for 3! of the 24 automorphisms.

`form` prints the shape with atoms named A, B, C and so on, preceded by a fingerprint of its certificate that is the same on every platform. `Boolean.And` and `Boolean.Or` print the same shape:

```
Shape 5266e42f98dc4183
[A.B.B.H.I] B.D,
[A.B.C.H.I] B.D,
[A.C.C.H.I] C.D,
[A.F.G.([E] H.I)] C.D,
[H.I.([E] H.I)] A.H.I,
```

`--library` loads rules-only files, `--target` adds a configuration every renaming must preserve, and `--fix` keeps an atom's name, in all three commands.

## Connections between fields

The webbook's [Connections](../index.html#connection) chapter compares programs written in different fields. Adding modulo 2, exclusive or and turning a card over are one shape; multiplying the odd numbers modulo 8, toggling two switches and the symmetries of a rectangle are another, with 6 symmetries; the quarter turns of a square are counting modulo 4 and have 2, so no dictionary joins them to the switches. Hovering a row of a dictionary lights that atom in every program, and the shape can be read in any member's names.

The page calls `compare` in the WebAssembly engine ([toolchain/browser/shape.rs](../toolchain/browser/shape.rs)). A request lists one to four programs and the answer lists the classes, each with its members, dictionary, shape, blocks, symmetries and group order. `//book:record` stores the answer for every preset so the book works from a file, and fails when a preset finds a different number of shapes than its page states.

## Verification

`//symmetry:test` compares the engine with brute force. On 400 random programs of up to 7 atoms with nested rule values, scopes and configurations, half of them closed under a random permutation so that they have symmetry, the group's order equals the number of permutations that leave the program unchanged, and every reported generator and block exchange is an automorphism. On 600 random pairs of up to 6 atoms, equal keys coincide exactly with the existence of a renaming found by trying every bijection, and every returned renaming maps one program onto the other. On 120 programs of up to 44 atoms and 30 rules, four random relabelings each give the same key, group order and canonical form. Vertex-transitive graphs, where refinement alone separates nothing, give the orders of their automorphism groups: 120 for the Petersen graph, 384 for the 4-cube, 144 for K3,4, 200 for two disjoint pentagons and 362,880 for K9. Comparing three programs gives the expected classes and a dictionary that maps one member onto the other.

`//command:test` runs the three commands. `//toolchain/browser:test` checks the WebAssembly comparison, including located errors, and that its dictionary equals the native command's, since the engine there runs on 32 bits. `//toolchain/browser:check` drives the chapter in a browser, from recorded runs and live.

## Survey

On 2026-09-25 the tool analyzed every source file on its own and every assembled test with its libraries and targets. It changed nothing; these are its findings.

Four pairs of the 205 tests were isomorphic with their libraries and targets:

| Tests | Renaming |
| --- | --- |
| `polarity.3` and `sign` in [program/ternary/case](../program/ternary/case/) | identical sources |
| `addition.check` in [program/association](../program/association/) and `sum.check` in [program/natural](../program/natural/) | identical once lowered |
| `annihilation.left.proof` and `annihilation.right.proof` in [theorem/ring](../theorem/ring/) | (Left Right) |
| `converse.proof` and `order.proof` in [theorem/lattice](../theorem/lattice/) | (Join Meet) (X Y) |

The last two are dualities: right annihilation is left annihilation in the opposite ring, and the two lattice theorems are each other's order duals. Each could be proved by renaming the other.

The 205 tests took under a second in total, a process each. 154 needed one search node, since refinement and blocks alone distinguished every atom, and none needed more than 10. The largest, `//program/circuit:divide.check`, has 3,630 rules and 713 atoms and takes 0.06 seconds.

Many library modules have symmetries that survive loading their dependencies. `Boolean.Or` is `Boolean.And` with `True` and `False` exchanged, De Morgan's duality, and `Boolean.Not` is its own dual. `Ternary.Compare` is unchanged by (Left Right)(Less Greater) and by (0 2)(Less Greater). The carry algebra is self-dual: `Carry.Combine` is unchanged by exchanging `Kill` and `Generate`, and `Carry.Evaluate` by that together with (0 1). `Pair.Choose`, `Selection.Check`, `Selection.Count` and `Ternary.Select` each exchange Booleans or trits together with their operands or answers. `Natural.Copy` treats all three digits alike, and `Natural.Trim` exchanges only 1 and 2. `Carry.Equal` is `Ternary.Equal` with the statuses named as trits. The laws are local: the whole library loaded as one program, 1,229 rules over 260 atoms, has no automorphism but the identity. Exchanging `True` with `False` fixes `Boolean.Not` but breaks `Boolean.Equal`, which answers `True` for both equal pairs. Loading dependencies matters too: [vector/merge.particle](../library/vector/merge.particle) alone is unchanged by exchanging `Up` with `Down` and `Less` with `Greater`, but not with `Natural.Compare` loaded beneath it.

Without their libraries, more source files coincide: `polarity.10` and `polarity.14` differ only by `Multiply → Divide`, and several fixtures in `program/association` are one shape. They are not redundant. The expression library tells `Multiply` from `Divide`, and the fixtures are targets for programs that tell their atoms apart.

51 tests had symmetries beyond their blocks. Some are properties of the claim: `theorem/natural:reversal` treats all three digits alike, `theorem/natural:trim` exchanges only 1 and 2, and the relation theorems exchange `Left` with `Right`. Many derived proofs share (True False)(Holds Counterexample), which comes from the case rules they load but never use.

Two optimizations the survey measured are not worth building. Contracting blocks into single atoms would remove 0.1% of atom occurrences across all tests, because an atom that always travels with another inside one module rarely does once the libraries are loaded. Exploring one configuration per orbit of the automorphism group would help only programs whose symmetries move reachable configurations; in the proofs explored in full, the symmetries mostly come from the unused case rules and move nothing. The learner's exhaustive `solve` could enumerate programs up to renaming of the atoms no example mentions, but its schedule breaks ties by atom identity; a schedule that broke ties by shape would allow it.

## Reproduction

```sh
bazel test -c opt //symmetry:test //command:test //toolchain/browser:test
bazel test -c opt //toolchain/browser:check
bazel run -c opt //command:photonic -- compare $(git ls-files '*.particle' '*.wave' | sed "s|^|$PWD/|")
```

The last command prints every class of isomorphic source files and fails, because the files fall into many classes.
