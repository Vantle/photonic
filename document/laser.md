# Laser

Laser is Photonic's second engine for exploring every future of a program, beside the interpreter in [language/runtime.rs](../language/runtime.rs). It lives in [language/laser.rs](../language/laser.rs) and [language/laser/](../language/laser/). Both engines read the same programs and apply rules with the same kernel, and a closed exploration reaches the same configurations, the same events and the same answers on either. Laser is built to be fast where exploration is large: it names configurations by their parts instead of searching for the canonical form of each, remembers what each rule does to those parts, and carries matches back along events in rounds that run in parallel. The interpreter stays the reference, and direct paths stay with its scheduler.

## Using it

| Where | How |
| --- | --- |
| `photonic run` | `--engine laser` lists the configurations Laser reaches, and `--plain` those of every schedule of plain events; `--worker` sets its threads and `--json` prints its report. |
| `photonic prism` | `--engine laser` answers reached, unreachable or unknown for an exact target from Laser's exploration, and `--plain` from every schedule of plain events. `--path` follows the interpreter's scheduler, so it refuses an engine and plain mode. |
| Spectrum | Every question takes `engine: laser` in exhaustive mode, and every verb takes `--engine laser`; see [Spectrum](spectrum.md#recordings). `mode: plain`, or `--plain`, explores every schedule of plain events on Laser, and with `engine: metal`, or `--engine metal`, through the program's [net of parts](#nets-of-parts) on the GPU through [Metal](#the-metal-backend), for `explore` and `check`. `check --plain --engine metal --end` with `--exact` and `--preserve` asks what `photonic_test(every = True)` checks. |
| `photonic_test` | Every exhaustive case also runs on Laser and fails unless Laser gives the interpreter's answer, or the interpreter's exploration stayed open and one of the two answers is unknown. `every = True` runs Laser alone and requires every schedule of plain events to end exactly at a target; see [every schedule](#every-schedule). |

```sh
bazel run -c opt //command:photonic -- run program/language/inference.wave --engine laser
bazel run -c opt //command:photonic -- explore program/language/conjunction.wave --engine laser
```

## The contract

For an exploration that closes, both engines agree on:

- the configurations reached, each in the interpreter's canonical form;
- the events, each as its source, target, rule and binding: the coherences it binds, the occurrences its input takes, which of those it matches exactly, and the rule occurrence it reads;
- which events are inferred, and which configurations and events are supported;
- every Prism verdict and every Spectrum claim;
- each event's place map, which says where every occurrence after the event came from, up to the configuration's automorphisms.

A configuration can hold interchangeable occurrences, three equal `X` in one coherence for instance. Each engine's canonical labeling settles on one order for them, not necessarily the same one, so the two place maps of an event can differ by an exchange of equal occurrences. Agreement compares place maps by what every automorphism keeps: each place's kind, its occurrence and the occurrences beside it.

The engines differ in what depends on the order of work. Each numbers configurations and events in the order it finds them, and Spectrum renumbers both canonically, so a closed exploration names the same handles on either; their keys differ. When a match reached an inferred event along several walks, each reports its own deduction: Laser gives the shortest walk among the traces that identify the event, each followed back along the crossing that first carried it. Each counts its own work and retained records. An exploration stopped by a budget ends at a different point on each engine, but a definite answer never contradicts the other engine's: reached needs a supported configuration, and support only grows as exploration continues, while unreachable needs a closed exploration.

## How it explores

**Configurations by their parts.** A configuration splits into components: worlds and frames other than the root, joined by shared occurrences, frame links and captures. The root frame is a hub that every component may refer to without joining them. Each component is extracted with a stand-in root and named by the canonical form of what it holds, its kind; the root is named by its own canonical form, or, when it shares an occurrence with a component or refers to one of its frames, the whole configuration is named as one. A configuration's makeup is its root and its sorted kinds. Equal configurations have equal makeups, so Laser finds a configuration it has seen by its makeup without searching for the canonical form of the whole.

**Transitions remembered.** An event reads and changes only the components holding what it binds, the frame where it fires, the frame that owns its rule and the root. Laser applies the rule to those parts alone, with the interpreter's kernel, and keys the result by the root, the touched kinds in order and the binding laid out over them. Every later event with the same key reuses the result: the untouched components keep their kinds, and the new makeup is the old one with the touched kinds replaced. A configuration whose root ties to a component, a result whose root does, and a rule owned through a capture are applied to the whole configuration instead.

**Matches carried back.** Inference lets a rule fire at a configuration on what that configuration becomes: a match found in a later configuration, carried back along the events between them, binds the occurrences it came from. Laser scans each new configuration for its own matches, which it keeps as traces, and carries every trace back across every event that reaches its configuration, once for each trace and event. A carried trace lands on a trace the event's source already has or becomes a new one there. Every application a trace identifies fires once; an event that a configuration's own match identifies is direct, and one that only carried traces identify is inferred.

**Rounds.** Each round scans the new configurations, carries traces across the events they have not crossed, and fires the applications they identify. Scanning, carrying, applying and naming run in parallel, and every result is merged in a fixed order, so the exploration and its report are the same for any number of workers. A round carries traces and applies events in batches, so what waits to be merged is one batch rather than a round; each batch finds what the batches before it made, which gives exactly the traces, positions and events a single batch would. A carried trace takes each of its places across the event to the place it came from, and most places come from one; a set of places, or a place that came from several, takes the image the event's table learned, so a trace whose image the table lacks waits until the batch has learned every image its waiting traces need. Traces carried along different walks often hold equal captures, which they share.

**Closing.** When no work remains, three passes finish the exploration. The first marks as direct every inferred event that a match found at its own source still derives through a cycle of crossings. The second establishes support in parallel rounds, the least set closed under the interpreter's rules: the start, a configuration's own matches, a trace carried across a supported event from a supported trace, an event identified at a supported source by a supported trace, and a configuration a supported event reaches. The third records each inferred event's deduction. The traces are then released; each event keeps its passage, the map from the places after it to the places before it, so place maps are answered later.

**Reports.** Laser renames each configuration it reports into the interpreter's canonical form, and every event's places with it, so reports, targets and Spectrum answers read alike on both engines.

## Plain exploration

`Laser::plain` explores every schedule of plain events: it fires only what each configuration's own matches identify and carries nothing back, so it has no inferred event. Its configurations are those that plain events reach from the start, and its events are the full exploration's events that a match at their own source identifies. Inference lets a rule consume what a configuration will become, so an inferred event can strand a run that every plain schedule would finish: in `Claim, [Claim] P.Work, [Work] Done, [P] X`, the rule `[P] X` can consume `Claim` before `Work` exists, and `inevitable Done` fails, while every plain schedule reaches `Done`. Plain mode is how Spectrum asks whether every order of rule applications reaches a result.

```sh
bazel run -c opt //command:photonic -- check "$(bazel info -c opt bazel-bin)/theorem/boolean/identity.proof.case.json" --plain --inevitable Theorem --outcome Theorem
```

### Reduced exploration

`Laser::reduced` explores the plain schedules through fewer interleavings. At each configuration it fires the events of one part whose events commute with every other event, now and after any events elsewhere, and leaves the rest for later configurations. The part is a coherence or a scope:

- A coherence qualifies when every event that binds it binds it alone, takes nothing else, reads nothing else but live rules and does not return from its scope; when no event of its frame binds nothing, so consuming it cannot close the frame under another event; and when no rule with several input coherences, live or held as a value, has an input coherence that names only what it holds, so nothing can ever join it with another coherence.
- A scope qualifies with every frame it encloses, the frames whose enclosing or defining frame it encloses, when no occurrence outside captures one of them. Nothing outside can bind their occurrences, add to them or apply their rules, and their events change only them, adding coherences and scopes to the scope's enclosing frame at most.

Among the parts with events it prefers work inside scopes to coherences of the root, then the part with the fewest events, so a run finishes what it started before it starts more. Every event of such a part stays enabled until it fires and commutes with every event outside it, so this is a persistent-set reduction: every configuration where a plain run ends is still reached, and a run can go on forever exactly when the reduced exploration has a cycle. A configuration where a rule applies through a capture fires everything. A rule whose input names a rule that is live in some scope could consume a live rule of any frame, even the root, and change what every event may do, so a program with one is explored without reduction. `laser::test` and the census check reduced explorations against plain ones: the same end configurations and cycles, and only plain configurations and events.

### Every schedule

`photonic_test(every = True)` checks that every schedule of plain events ends exactly at one of the test's targets. Laser's reduced plain exploration must close within the test's limits, no cycle of configurations may be reachable, since a run around one goes on forever, and every configuration where a run ends, which no event leaves, must be a target. The test prints how many configurations and events it explored, how many configurations runs end at, and how many of those are not targets. `theorem(every = True)` adds the check to a theorem as the test named after it with `.order`.

Explored with up to 131,072 configurations, 69 of the 80 theorems end exactly at `Theorem` in every plain schedule, and each carries an `.order` test. The reduction closes Peirce's law in 74 configurations where the plain exploration needs 130,322, and lookahead in 1,228 where it needs 124,636; the largest reduced explorations are van der Waerden's progression of nine, with 84,988 configurations and 125,947 events, associativity at every width, with 15,550 and 21,327, and the triangles of K6, with 14,454 and 20,553. Four theorems have schedules that end without `Theorem`: group cancellation, lattice transitivity, and left and right ring annihilation. In left annihilation, the rules that chain two equations through a shared side can combine them in an order that never yields 0 · X = 0. Natural successor closes at 87,539 configurations, and 872 of the 873 configurations where its runs end hold something beside `Theorem`, so its path proof speaks for the scheduler's order alone. The other six, natural addition, subtraction, opening, ending, reversal and trim, stay open within 131,072 configurations and their occurrence limits.

## Nets of parts

A program is a net when no configuration it reaches ties its root to a component. [laser/net.rs](../language/laser/net.rs) names each configuration, a marking, by its root and its components' sorted kinds, as Laser does, and knows each plain event by the parts it touches: an event binds one component with the rules its root sees, binds coherences of the root frame that lie in several components, or binds nothing but the root. So the events of a part are found once, by the interpreter's matcher on the part alone, and applied once, and exploring asks only tables. A table entry says what a part's events make of it: the root after them, the kinds that replace the part's, and how many distinct events do that. A component's events repeat for every copy of its kind, and an event joining several components repeats for every choice of copies, so a successor carries how many events lead to it.

`Net::explore` explores every schedule of plain events of a net breadth first on the host, numbering configurations in the order it finds them, and looks for a run that goes on forever when `Cycle::Find` asks it to. It closes with Laser's plain exploration: the same configurations, events, end configurations and cycles, which `laser::test` and the census check. A program whose root ties to a component is not a net, and `Net::new` reports it.

Grounding a part reads every match the matcher finds on it, one step of work each, and reading the tables afterwards costs nothing, so a net's work is the work its parts took. A part can hold millions of matches when a rule chooses among many equal atoms of one particle. The work budget stops the exploration before the first configuration whose parts it cannot ground, and leaves it open; the configuration limit bounds how many configurations it expands.

## The Metal backend

The [wave](../wave/) crate explores a net's plain schedules on the GPU through Metal, with the [metal](../metal/) runtime it shares with the learner's [gpu](../gpu/) crate. `wave::engine::Engine::new` gives the engine on the system's GPU, or none where no GPU's kernels can reach memory through device addresses, which Metal 3 brought. `wave::engine::Engine::explore` takes the net, the work budget, the limits and the same `Cycle` choice, and returns what `Net::explore` returns: whether the exploration closed, how many configurations and events it found, the configurations where runs end, when asked whether a run can go on forever, and the work its grounding took. Both agree number for number. They number configurations in the order one thread searching breadth first finds them, ground parts in the same order and so number kinds alike and take the same work, refuse the same configurations and applications under every limit, and stop before the same configuration under every budget; past the configuration limit, both reach only configurations they already know.

**Markings.** Each marking is stored once, as its root, its number of kinds and its sorted kinds, in an arena of segments that never move. The first GPU use of a buffer commits all of it, so the arena grows by adding segments rather than by copying, and the kernels reach segments through their device addresses. A table finds a marking by a 64-bit hash: the scrambled sum of a term for its root and a term for each kind. An event changes the sum by the terms of what it removes and adds, so a successor's hash follows from its source's sum and its table entry. The table is made of buckets of eight slots, one cache line each; a slot holds a marking's number and a check from its hash, so a probe reads one line and compares words only when the check matches.

**Passes.** A window of markings is counted first: each marking's successors from the tables, then a running sum that places them. A pass then decides up to 2<sup>23</sup> candidates in one command. Expanding records each candidate as its source, its entry and how many copies of the consumed kind it could take, never as the successor's words, and checks the limits. Inserting finds each candidate's marking: an established one it equals word for word, or a slot it claims, tagged with its index, or shares with an equal candidate; the lowest index keeps the slot, so numbering never depends on timing. Each threadgroup of candidates sums its winners and their words, a running sum over the groups places each group, and placing ranks each winner within its group, numbers and writes the new markings and points every candidate at its marking. A pass the configuration limit can cut short makes room only for the groups before the first whose winners it refuses all, and sums its events once every candidate points at its marking. The same command counts the next window, so a small breadth-first level costs one command; only new markings that overflow the arena's last segment take a second.

**The host's share.** Grounding stays on the host. When a window holds a marking whose parts the tables lack, or whose kinds could fill every input of a rule joining several coherences, the host grounds it as its own net would on expanding it, counts again if it must, and finds the successors of joining events, which the pass decides with the rest. A marking the budget cannot ground ends its window, which is counted again without it and is the last. When asked whether a run can go on forever, the host keeps every successor's marking as an edge, and inserting flags a successor that is a marking found no later than its source. Every cycle holds such an edge, so the host searches the edges for a cycle only when one is flagged.

Spectrum's `metal` engine runs it: `explore` and `check` with `--plain --engine metal` give the counts, the configurations where runs end and whether a run can go on forever, and answer `end` and `outcome` claims, keeping four bytes a successor and eight a configuration to look for cycles. An exploration a limit leaves open says a run can go on forever when it finds a cycle, and otherwise leaves it unknown, since what the limit refused could close one.

```sh
bazel run -c opt //command:photonic -- explore program/language/inference.wave --plain --engine metal
bazel run -c opt //benchmark:wave -- --family task --count 14
bazel run -c opt //benchmark:wave -- --family dial --count 12 --compare --worker 16
```

The macOS sandbox denies Bazel's test actions the GPU, so `//wave:test`, `//metal:test` and `//gpu:test` run outside it, while their builds stay sandboxed. Elsewhere `Engine::new` gives no engine, and metal explores on the host's net.

## Budgets

Laser takes the interpreter's limits. The configuration, coherence, occurrence and scope limits block an application whose result exceeds them, as the interpreter does. Work counts one item for each configuration scanned, each trace carried across an event and each application tried, so a work budget stops the two engines at different points. Laser's retained records are its configurations, events and traces, and it pauses when they reach the record budget. Every budget defers work rather than dropping it: running again with a larger budget resumes where the last run stopped.

## Verification

`laser::test` in [language/test/laser.rs](../language/test/laser.rs) compares Laser with the interpreter on programs with scopes, captured rules, inference and rule values, and on the dial and diner families; it checks runs left open, identical results for one and four workers, resuming after configuration and record limits and a step at a time under limits that rise, Prism verdicts, and that every deduction walks from its event's source. Each comparison covers configurations, events with their bindings, inferred events, support and place maps. `//spectrum:test` checks that both engines give the same counts, claim answers and verdicts, and `//command:test` runs the commands with Laser.

`laser::test` also checks that every plain exploration is exactly the part of the full one that matched events reach, and where plain runs end and when one can go on forever, including runs that meet again after different orders, and that the host's net mirrors every plain exploration of 35 programs with scopes, joins and cycles. `wave::test` compares the GPU with the host's net on the dial, diner and task families and on programs with scopes and joins, under the default limits and configuration, coherence, occurrence and scope limits that stop them midway, with the default window, pass and segment sizes and with windows of three markings, passes of seven candidates and segments of 64 words. Every comparison requires the same configurations, events, cycles and end configurations, kind for kind. `//benchmark:agreement` compares the engines on every program in the repository, each once, checks every plain exploration of a program Laser closes against its full one, checks every reduced exploration against its plain one where the plain one closes, and explores every program's net on the host and, where there is a Metal device, on the GPU, which must agree number for number: the webbook's examples and workbench presets, every `photonic_test` case and assembled program of the standard library, the programs and the theorems, the programs written in the language tests, the reference programs and the dial and diner families. With `--reduction` it explores only the plain and reduced schedules and the nets, which checks the reduction on programs too large for the interpreter, such as the theorems with `--configuration 131072`.

```sh
bazel run -c opt //benchmark:agreement -- --root "$PWD" --bin "$(bazel info -c opt bazel-bin)"
```

`//benchmark:fuzz` writes a random program for each seed from the language's own forms: dotted particles over four atoms, rules with one or two inputs, empty inputs, outputs that are empty, single, grouped or scoped, rule values held as fields, and scopes. It makes every check the census makes on each, and one more: Laser explored a step at a time under configuration limits that rise, so blocked identities are retried while new traces arrive. With `--worker 4` it also explores each program of up to 10,000 events on four workers and requires the report of one worker. It reports each disagreement, and each panic, with its seed and program, and fails if there is any; `--write` prints the programs instead. A rule that chooses among many equal atoms of one particle gives a few configurations millions of events, so the engines run under budgets and a record limit, and the programs run in parallel, so memory grows with the threads.

```sh
bazel run -c opt //benchmark:fuzz -- --count 2000
```

## Measurements

Two families in [language/family.rs](../language/family.rs) grow as large as wanted: `dial(n)` turns n dials through Zero, One and Two, and `diner(n)` seats n diners around a table, each needing the forks on either side to eat. Both infer: six dials reach 13,122 events, 8,748 of them inferred. Timings are medians of three runs of `//benchmark:laser` built with `-c opt` on the development machine, and memory is the process's peak resident size.

| Program | Configurations | Events | Interpreter | Laser |
| --- | ---: | ---: | ---: | ---: |
| 4 dials | 81 | 972 | 50.1 ms | 2.3 ms |
| 6 dials | 729 | 13,122 | 9.64 s | 32.3 ms |
| 3 diners | 62 | 525 | 82.6 ms | 4.0 ms |
| 4 diners | 238 | 3,128 | 6.58 s | 24.9 ms |

Beyond these the interpreter takes minutes and tens of gigabytes, so Laser runs alone.

| Program | Configurations | Events | One worker | 16 workers | Memory |
| --- | ---: | ---: | ---: | ---: | ---: |
| 8 dials | 6,561 | 157,464 | 0.49 s | 0.19 s | 0.32 GB |
| 9 dials | 19,683 | 531,441 | 1.93 s | 0.70 s | 1.00 GB |
| 10 dials | 59,049 | 1,771,470 | 7.33 s | 2.60 s | 3.75 GB |
| 5 diners | 937 | 17,365 | 0.16 s | 0.07 s | 0.09 GB |
| 6 diners | 3,697 | 89,880 | 1.01 s | 0.31 s | 0.39 GB |
| 7 diners | 14,583 | 440,496 | 5.78 s | 1.46 s | 1.73 GB |

Memory grows with events: every event keeps its identity and its passage, and until the exploration closes each trace keeps where it landed across every event it crossed. Small programs pay for work the interpreter shares between configurations, since Laser scans each configuration from scratch and names every component it makes: the 19 closed reference programs in [reference.json](../language/test/reference.json), with 2 to 29 configurations, take 22 to 83 percent longer on Laser, from 14.2 µs against 8.4 µs for the smallest to 409 µs against 319 µs for the largest.

Theorems explored in every order are heavier. With inference, a match found late can be carried back to many configurations along many walks, and each walk can bind different occurrences: the proof of natural restoration, stopped at 4,096 configurations, holds 3.3 million traces and crosses 105.8 million times to reach its 117,419 events, taking 40.0 s on one worker and 6.5 s on 16, with a peak of 5.3 GB on one worker and 6.3 GB on 16.

Of the 470 distinct programs the census gathers, 299 close on the interpreter within the default budget, and Laser agrees on all 299. Every one of the 303 plain explorations of programs Laser closes is the matched part of its full exploration, and each of the 338 plain explorations that closes has a reduced exploration with the same end configurations and cycles, 60 of them smaller. The host's net mirrors all 338, and all 470 programs are nets that the GPU explores exactly as the host does, those that stop at the configuration limit included. With `--reduction --configuration 131072`, the 48 theorems whose plain explorations close within that limit agree with their reduced ones as well. Four more close only on Laser: the proof of Boolean domination, 6 dials, and 4 and 5 diners. With the configuration limit raised to 300,000, Laser also closes six more theorem proofs in every order, which the interpreter leaves open within its default limits: Boolean complement (4,357 configurations), identity (16,513) and idempotence (22,953), componentwise reflexivity (9,478), relation trichotomy (5,755) and sum irreflexivity (6,401).

Nets explore every plain schedule fastest on the GPU. `task(n)` runs n tasks from pending to running to done, so its schedules end; dials turn forever. Times are medians of five runs of `//benchmark:wave` on the development machine, against the host's net and Laser's plain engine on 16 workers:

| Program | Configurations | Events | Metal | Host net | Laser plain |
| --- | ---: | ---: | ---: | ---: | ---: |
| 10 tasks | 59,049 | 393,660 | 6.7 ms | 54.6 ms | 0.33 s |
| 12 tasks | 531,441 | 4,251,528 | 13.6 ms | 0.61 s | 3.52 s |
| 14 tasks | 4,782,969 | 44,641,044 | 65.1 ms | 6.94 s | |
| 16 tasks | 43,046,721 | 459,165,024 | 0.50 s | | |
| 10 dials | 59,049 | 590,490 | 6.0 ms | 71.5 ms | 0.47 s |
| 12 dials | 531,441 | 6,377,292 | 15.3 ms | 0.81 s | 4.88 s |
| 14 dials | 4,782,969 | 66,961,566 | 80.6 ms | 9.98 s | |
| 16 dials | 43,046,721 | 688,747,536 | 0.61 s | | |

Sixteen tasks explore at 86 million configurations a second and sixteen dials at 71 million. Theorems explore too: the plain schedules of the natural-number `ending` proof pass 50 million configurations in 1.3 s, with every joining event found on the host, and have not ended there. Every command waits for the GPU, so a breadth-first level costs at least one command's round trip, and nets of a few thousand configurations explore as fast on the host. The host also finds every successor of an event that joins several components, and every event of the diners joins two; their plain schedules stay small, 478 configurations for seven diners, which the host's net explores in 2.6 ms and the GPU in 6.1 ms.

## Not yet built

Laser does not follow direct paths; `--path` and Spectrum's `path` mode use the interpreter's scheduler. Deductions are each engine's own; shortest derivations shared by both would let `inspect` and `cause` read alike on either. The webbook's WebAssembly engine runs the interpreter only. The Metal backend explores nets and keeps no events, so of Spectrum's questions it answers `explore` and `check` alone, and of the claims `end` and `outcome`; it decides claims on the host from the ends and cycles it found, grounds and joins on the host, looks for cycles on the host from every edge, and does not infer.
