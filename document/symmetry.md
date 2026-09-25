# Program symmetry

An atom is a name and nothing more: rules compare atoms only for equality. Renaming the atoms of a program, one to one, renames every configuration it reaches and every event between them. What a program means is therefore the pattern its names form, its shape, and programs that differ only by their names are one program, whatever field their names come from.

The [symmetry](../symmetry/) package decides this. For any program, together with a target or with atoms whose names must stay, it computes a canonical form shared by every renaming, the renaming between two programs when one exists, and the automorphism group: every renaming that leaves the program unchanged. It also finds local symmetries, groups of rules that repeat under other names inside one program when the program as a whole has no symmetry. `photonic compare`, `photonic symmetry` and `photonic form` expose it, the [Connections](../index.html#connection) chapter of the webbook runs it in the browser, and the workbench and the Lightbox color every graph they draw by the symmetries of the program they run.

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

## Local symmetry

A program with no symmetry as a whole can still repeat itself. The integer addition library treats a positive and a negative left operand with six rules each, the same rules with `Positive` and `Negative` exchanged, but when the signs differ it swaps the operands only if the left one is negative, so no renaming of the whole library leaves it unchanged. A local symmetry is such a repetition: a set of statements, each a rule or an initial coherence, that occurs two or more times, every copy a renaming of the first. The copies are disjoint, each renaming fixes the atoms the copies share, and the atoms it moves are the ones that tell the copies apart. Finding them is a subgraph problem: the copies are isomorphic subgraphs of the program's graph, one for each copy.

**Seeds.** Every statement gets a canonical form of its own from the search above, with the atoms named by `--fix` kept apart. Statements of equal form are the seeds: each is a copy of a one-statement pattern.

**Growth.** A pattern grows one statement at a time along its list of copies, the way the frequent subgraph miners gSpan and GraMi grow patterns with their embeddings. The first copy adds a free statement that shares an atom with it. Every other copy looks up, through an index from atoms to statements, a free statement of the same form attached to its own copy the same way: its atoms already in the copy take the places the first copy's atoms hold, and its other atoms are new to the copy. A statement's own symmetries let one form attach in several ways, so the match tries each automorphism of the new statement's form, up to 4,096 of them, with its blocks free, as the pattern-aware matchers Peregrine and GraphPi account for a pattern's symmetries. Copies that cannot follow drop out. Growth continues while (copies − 1) × size, the number of vertices the repetition accounts for beyond one copy, increases.

**Selection.** Grown patterns compete by that score in a lazy greedy, like the compression heuristic of SUBDUE and the abstractions Stitch learns: the best pattern whose statements are all still free is accepted and takes its statements, and a pattern that lost a statement to another regrows from the statements it has left. The copies of a one-statement pattern must connect through shared atoms, or they would only say that two statements have the same form; copies that fall apart split into patterns of their own. Statements that a global symmetry moves are left out, since the global symmetry already says more about them. The search is bounded by a budget of match attempts, so every program gets an answer.

## Three kinds of symmetry

| Kind | Relates | Found by |
| --- | --- | --- |
| global | the atoms that renamings of the whole program exchange, and the statements they move | the automorphism group |
| local | the copies of a repeated group of statements, and the atoms that differ between copies | pattern growth |
| block | atoms that always appear together | contracting twins |

Global classes join the orbits of atoms and statements that the symmetries exchange together; a local class is one pattern; a block is its own class. The WebAssembly engine runs the analysis with every exploration ([toolchain/browser/analysis.rs](../toolchain/browser/analysis.rs)) and returns the classes with their atoms and the names of their rules, as events name the rules they apply. The workbench and the Lightbox color the graphs by kind. An atom colors its tokens in the state graph, the hypergraph and the inspectors, and a rule colors every event it causes, including events of the rules nested in its scopes. An atom in two classes takes the first kind in the order above, and a rule name that statements of different classes share stays plain. Hovering or selecting a symmetry in the panel lights its members and dims the rest.

The three colors are the yellow, magenta and green of a validated categorical palette, checked with every pair of colors adjacent, against the book's surfaces in both themes and against the blue that marks selections. Under simulated protanopia and deuteranopia every pair stays apart except local and block in the dark theme, which sit in the floor band; blocks therefore also wear a dashed ring. Labels keep the ink colors, and every symmetry in the panel names its kind in words.

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

`symmetry` prints the order of the automorphism group, the blocks, every symmetry when the group has at most 64 elements and generators otherwise, each orbit of statements that the symmetries exchange, and each local pattern with the atoms that differ between its copies:

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

`Boolean.And` has no symmetry beyond its blocks, but one law holds for each value on its own: a conjunction of a value with itself is that value.

```
9 atoms, 5 rules, 4 automorphisms
Block And.Boolean
Block Empty.Reduce
Pattern of 1 statements in 2 copies
  True
    [Function.Boolean.And.True.True] True.Return
  False
    [Function.Boolean.And.False.False] Return.False
```

Each copy lists the atoms that differ from the other copies, then its statements. Particles print their atoms in the order the program first names them.

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

Local symmetry is checked on 300 programs that each plant a group of one to three random rules in two to four copies, sharing up to two atoms, among unrelated rules. Every reported pattern is exact: its copies are disjoint, each copy's atoms are distinct, no copy holds a statement a global symmetry moves, the copies of a one-statement pattern connect through shared atoms, and renaming the first copy by the positions of its atoms gives every other copy statement by statement. The search reports more than 60 patterns across the 300 programs; planted copies that no other rule tells apart are global symmetries instead. A program with a global orbit, a local pair and a block reports one class of each kind.

`//command:test` runs the three commands, including a pattern. `//toolchain/browser:test` checks the WebAssembly comparison, including located errors, and that its dictionary equals the native command's, since the engine there runs on 32 bits; it also checks the classes that exploration reports and that they name rules as events do. `//toolchain/browser:check` drives the chapter, the workbench and the Lightbox in a browser, from recorded runs and live, including the colors, the lighting and the switch that turns the colors off.

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

Local symmetry finds relations that no renaming of a whole library shows. In `Natural.Add` assembled with its dependencies, the column machine's addition and borrowing modes are the same ten statements, with `Add`, `Read` and `Yield` renamed `Borrow`, `Peek` and `Seen`. `Natural.Compare` handles each of the three digits with the same 19 statements, the other two digits permuted to match. `Vector.Sort` merges in both directions with the same 61 statements: exchanging `Up` with `Down`, `Less` with `Greater`, `Heap` with `Pile` and the digits 1 with 2, and naming `Rise` as `Fall`, turns one direction into the other. `Integer.Add` handles a positive and a negative left operand with six rules each. The analysis of `Expression.Evaluate`, 751 rules, takes 0.22 seconds and finds 132 patterns; `//program/circuit:divide.check`, 3,630 rules, takes 1.3 seconds.

Two optimizations the survey measured are not worth building. Contracting blocks into single atoms would remove 0.1% of atom occurrences across all tests, because an atom that always travels with another inside one module rarely does once the libraries are loaded. Exploring one configuration per orbit of the automorphism group would help only programs whose symmetries move reachable configurations; in the proofs explored in full, the symmetries mostly come from the unused case rules and move nothing. The learner's exhaustive `solve` could enumerate programs up to renaming of the atoms no example mentions, but its schedule breaks ties by atom identity; a schedule that broke ties by shape would allow it.

## Reproduction

```sh
bazel test -c opt //symmetry:test //command:test //toolchain/browser:test
bazel test -c opt //toolchain/browser:check
bazel run -c opt //command:photonic -- compare $(git ls-files '*.particle' '*.wave' | sed "s|^|$PWD/|")
```

The last command prints every class of isomorphic source files and fails, because the files fall into many classes.
