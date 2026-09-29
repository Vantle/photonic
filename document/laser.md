# Laser

Laser is Photonic's second engine for exploring every future of a program, beside the interpreter in [language/runtime.rs](../language/runtime.rs). It lives in [language/laser.rs](../language/laser.rs) and [language/laser/](../language/laser/). Both engines read the same programs and apply rules with the same kernel, and a closed exploration reaches the same configurations, the same events and the same answers on either. Laser is built to be fast where exploration is large: it names configurations by their parts instead of searching for the canonical form of each, remembers what each rule does to those parts, and carries matches back along events in rounds that run in parallel. The interpreter stays the reference, and direct paths stay with its scheduler.

## Using it

| Where | How |
| --- | --- |
| `photonic run` | `--engine laser` lists the configurations Laser reaches; `--worker` sets its threads and `--json` prints its report. |
| `photonic prism` | `--engine laser` answers reached, unreachable or unknown for an exact target from Laser's exploration. `--path` follows the interpreter's scheduler, so it refuses an engine. |
| Spectrum | Every question takes `engine: laser` in exhaustive mode, and every verb takes `--engine laser`; see [Spectrum](spectrum.md#recordings). `mode: plain`, or `--plain`, explores every schedule of plain events on Laser. |
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

The engines differ in what depends on the order of work. Each numbers configurations and events in the order it finds them, so Spectrum handles and keys differ between them. When a match reached an inferred event along several walks, each reports its own deduction: Laser gives the shortest walk among the traces that identify the event, each followed back along the crossing that first carried it. Each counts its own work and retained records. An exploration stopped by a budget ends at a different point on each engine, but a definite answer never contradicts the other engine's: reached needs a supported configuration, and support only grows as exploration continues, while unreachable needs a closed exploration.

## How it explores

**Configurations by their parts.** A configuration splits into components: worlds and frames other than the root, joined by shared occurrences, frame links and captures. The root frame is a hub that every component may refer to without joining them. Each component is extracted with a stand-in root and named by the canonical form of what it holds, its kind; the root is named by its own canonical form, or, when it shares an occurrence with a component or refers to one of its frames, the whole configuration is named as one. A configuration's makeup is its root and its sorted kinds. Equal configurations have equal makeups, so Laser finds a configuration it has seen by its makeup without searching for the canonical form of the whole.

**Transitions remembered.** An event reads and changes only the components holding what it binds, the frame where it fires, the frame that owns its rule and the root. Laser applies the rule to those parts alone, with the interpreter's kernel, and keys the result by the root, the touched kinds in order and the binding laid out over them. Every later event with the same key reuses the result: the untouched components keep their kinds, and the new makeup is the old one with the touched kinds replaced. A configuration whose root ties to a component, a result whose root does, and a rule owned through a capture are applied to the whole configuration instead.

**Matches carried back.** Inference lets a rule fire at a configuration on what that configuration becomes: a match found in a later configuration, carried back along the events between them, binds the occurrences it came from. Laser scans each new configuration for its own matches, which it keeps as traces, and carries every trace back across every event that reaches its configuration, once for each trace and event. A carried trace lands on a trace the event's source already has or becomes a new one there. Every application a trace identifies fires once; an event that a configuration's own match identifies is direct, and one that only carried traces identify is inferred.

**Rounds.** Each round scans the new configurations, carries traces across the events they have not crossed, and fires the applications they identify. Scanning, carrying, applying and naming run in parallel, and every result is merged in a fixed order, so the exploration and its report are the same for any number of workers. A round carries traces and applies events in batches, so what waits to be merged is one batch rather than a round; each batch finds what the batches before it made, which gives exactly the traces, positions and events a single batch would. Traces carried along different walks often hold equal captures, which they share.

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

## Budgets

Laser takes the interpreter's limits. The configuration, coherence, occurrence and scope limits block an application whose result exceeds them, as the interpreter does. Work counts one item for each configuration scanned, each trace carried across an event and each application tried, so a work budget stops the two engines at different points. Laser's retained records are its configurations, events and traces, and it pauses when they reach the record budget. Every budget defers work rather than dropping it: running again with a larger budget resumes where the last run stopped.

## Verification

`laser::test` in [language/test/laser.rs](../language/test/laser.rs) compares Laser with the interpreter on programs with scopes, captured rules, inference and rule values, and on the dial and diner families; it checks runs left open, identical results for one and four workers, resuming after configuration and record limits, Prism verdicts, and that every deduction walks from its event's source. Each comparison covers configurations, events with their bindings, inferred events, support and place maps. `//spectrum:test` checks that both engines give the same counts, claim answers and verdicts, and `//command:test` runs the commands with Laser.

`laser::test` also checks that every plain exploration is exactly the part of the full one that matched events reach, and where plain runs end and when one can go on forever, including runs that meet again after different orders. `//benchmark:agreement` compares the engines on every program in the repository, each once, checks every plain exploration of a program Laser closes against its full one, and checks every reduced exploration against its plain one where the plain one closes: the webbook's examples and workbench presets, every `photonic_test` case and assembled program of the standard library, the programs and the theorems, the programs written in the language tests, the reference programs and the dial and diner families. With `--reduction` it explores only the plain and reduced schedules, which checks the reduction on programs too large for the interpreter, such as the theorems with `--configuration 131072`.

```sh
bazel run -c opt //benchmark:agreement -- --root "$PWD" --bin "$(bazel info -c opt bazel-bin)"
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

Of the 467 distinct programs the census gathers, 296 close on the interpreter within the default budget, and Laser agrees on all 296. Every one of the 300 plain explorations of programs Laser closes is the matched part of its full exploration, and each of the 335 plain explorations that closes has a reduced exploration with the same end configurations and cycles, 60 of them smaller. With `--reduction --configuration 131072`, the 48 theorems whose plain explorations close within that limit agree with their reduced ones as well. Four more close only on Laser: the proof of Boolean domination, 6 dials, and 4 and 5 diners. With the configuration limit raised to 300,000, Laser also closes six more theorem proofs in every order, which the interpreter leaves open within its default limits: Boolean complement (4,357 configurations), identity (16,513) and idempotence (22,953), componentwise reflexivity (9,478), relation trichotomy (5,755) and sum irreflexivity (6,401).

## Not yet built

Laser does not follow direct paths; `--path` and Spectrum's `path` mode use the interpreter's scheduler. `photonic run` and `prism` do not offer plain mode. Handles are numbered in each engine's own order; a numbering and deductions shared by both engines would let their answers name the same handles. The webbook's WebAssembly engine runs the interpreter only. There is no GPU backend yet.
