# Program optimization learner

`//learning:learning` learns to optimize Photonic programs and to find them from tests alone. A program's rules are edited one structural step at a time and every edited program is executed exactly; a planner guided by a label-free transformer chooses the edits, exhaustive search proves small programs optimal, and guided search reaches past it. The objective keeps every example correct under all schedules while minimizing parallel time and description length, under a goal each task may set. The learner persists its network, task pool and discoveries, so every use continues its training.

Training runs self-play on the task pool, alone or in loops that generate new behaviors:

```sh
bazel run -c opt //learning:learning -- train --duration 3600
bazel run -c opt //learning:learning -- train --duration 1800 --task gnome.4x3 --task gnome.7x2
bazel run -c opt //learning:learning -- optimize --program "$PWD/program.particle" --input "$PWD/input.wave" --duration 600
bazel run -c opt //learning:learning -- improve --round 4
bazel run -c opt //learning:learning -- curriculum --duration 7200
```

Solving searches for the cheapest program of chosen tasks, or of a task defined by pairs of input and expected output files:

```sh
bazel run -c opt //learning:learning -- solve --task boolean.and
bazel run -c opt //learning:learning -- solve --blind --enumerate 5 --guide 10
bazel run -c opt //learning:learning -- solve --task gather.3x3 --processor 1
bazel run -c opt //learning:learning -- solve --name tag --input "$PWD/one.wave" --output "$PWD/one.result.wave" --input "$PWD/two.wave" --output "$PWD/two.result.wave"
```

Inspection reports the archive and checks it against the runtime:

```sh
bazel run -c opt //learning:learning -- status
bazel run -c opt //learning:learning -- status --task gnome.4x2
bazel run -c opt //learning:learning -- verify
```

`--task` gives the named tasks the `--focus` share of the episodes, half by default; training stops with an error when none of them can be trained on. `--focus` sets the same share for the tasks `optimize`, `improve` and `curriculum` focus on, and `--frozen`, which searches without training or saving the network, belongs to `train` and `optimize` alone, since the loops need a network that learns. `--duration` ends a session and bounds every evaluation in it, so a session stops soon after it instead of finishing a long evaluation; for `improve` and `curriculum` it bounds the whole run, down to each search, lesson and exam. `solve` searches every small program exhaustively for up to `--enumerate` seconds per task, a bound on each evaluation too, returns the cheapest and proves it optimal when the search completes; `--guide` adds guided search where the proof does not finish, `--show` prints the cheapest programs either search found, `--blind` ignores every known program, `--input` and `--output` define a new task by its tests alone, and `--processor` or `--size` set that task's goal or solve copies of the `--task` tasks under another goal, taking what they leave unset from each task's own goal or the default. A task that exhaustive search cannot take, with more than 24 atoms or no positive size weight, is reported and left to guided search; the archive is saved after every task. `status --task` prints a task's reference and best programs with their work, span, size and cost on every input, measured under `--processor` and `--size` when the task has no goal of its own, and `status` marks the programs proven optimal, or proven optimal among flat programs where the learner could also have built nested ones.

A task name becomes a file name, so it consists of letters, digits, `.`, `-` and `_` and starts with a letter or digit; the names, goals and curriculum level read from `pool.json` and `curriculum.json` are checked as strictly as the command line checks them. A task enters the pool only when it can be trained on, so `optimize` and `solve --input` refuse one that names too many atoms, whose reference is correct only under the verifying limits, or whose expected outputs hold more coherences or occurrences than the training limits admit, before adding it to the pool; a session skips a pooled task that fails the same checks.

`bazel run` starts in its runfiles directory; relative paths resolve against the directory the command was run from. The learner home defaults to `~/.cache/photonic/learning`; pass `--home` to use another.

Tasks no longer store how many hidden atoms they carry, and archived programs record the goal of their proof instead of a flag, so a home saved before either change fails to load with ``pool.json is malformed: unknown field `hidden` ``. Migrate it in place from the home directory, skipping `curriculum.json` where the home has none, and prove its programs again with `solve`:

```sh
jq 'map(del(.hidden))' pool.json > pool.new && mv pool.new pool.json
jq '.exam |= map(map(.task |= del(.hidden)))' curriculum.json > curriculum.new && mv curriculum.new curriculum.json
jq 'map_values(.best |= (if . then del(.proven)+{proof:null} else . end) | .partial |= (if . then del(.proven)+{proof:null} else . end))' archive.json > archive.new && mv archive.new archive.json
```

Archived proofs now also record what they cover, so an archive saved before fails to load with ``archive.json is malformed: unknown field `processor` ``. Every earlier proof covered the flat programs over its task's vocabulary, so migrate it in place from the home directory, and run `solve` again to restore full coverage wherever the learner searches only flat programs:

```sh
jq 'map_values(.best |= (if . and .proof then .proof |= {goal: ., coverage: "Flat"} else . end) | .partial |= (if . and .proof then .proof |= {goal: ., coverage: "Flat"} else . end))' archive.json > archive.new && mv archive.new archive.json
```

The same change renamed `train --grow` and `curriculum --train` to `--fresh`, the name `improve` already used for new behaviors; moved `--focus` from `optimize` to every command that trains; and removed `--frozen` from `improve` and `curriculum`.

## Layers

| Package | Responsibility |
| --- | --- |
| [code](../code/) | One label-free representation of the whole grammar: atoms, particles, rule values, bodies, programs, configurations and observations with introduction sharing, plus canonical identity. |
| [machine](../machine/) | Exact execution of flat programs: exhaustive exploration of every interleaving, seeded walks, and a maximal parallel schedule. |
| [language/execution.rs](../language/execution.rs) | The same queries for any program, built on the runtime's own transition kernel and dispatch network. |
| [translation](../translation/) | Conversion between `code`, runtime source values and Photonic text. |
| [network](../network/) | A pointer transformer with hand-derived gradients, AdamW, checkpoints and function-preserving growth. |
| [gpu](../gpu/) | The same network on Metal: batched inference and training with AdamW, checked against the CPU gradient. |
| [learning](../learning/) | Tasks and their goals, rule-tree navigation, edits and rule analogy, the objective and configuration distance, encoding, planning, self-play, the learned judge, lessons, training, exhaustive and guided solving, the curriculum, the archive and the command. |
| [random](../random/) | A seeded generator shared by the layers. |
| [hashing](../hashing/) | The fast deterministic hasher and mixing functions that `code`, the runtime and the analyzers share, so canonical keys agree everywhere. |

The network never sees a name. Atoms enter as identities drawn from a fresh random permutation every episode, as do rule, particle and example slots, so the only thing that can be learned is structure. Names return only when a program is written as Photonic text.

## Correctness

A program is correct on an example when every actual-execution interleaving from the input terminates in the expected observation. Photonic has no rule priority, so correctness cannot depend on a scheduler. The observation is the root coherences with their introduction sharing; `[A] (B, C)` applied to `A.X` produces two coherences that share one `X`, which differs from two independent `X` occurrences. Live rule occurrences are not observed, so an optimized program may consume or keep different rules.

Flat programs run on the machine. Programs with rule values or bodies run on the runtime's kernel through `photonic::execution`, which drains the dispatch network at each state and applies the single evaluation kernel; it does not perform source inference. A cycle, a state outside the admission limits, more than one terminal observation, or a wrong terminal makes an example incorrect. In training and solving, exploration has a state limit for each example and a budget of 8,192 states for all of a program's examples together; the verifying limits are larger and share no budget. When an example's exploration exceeds its state limit or what remains of the budget, seeded random walks decide it; once the budget is spent, each remaining example is decided by the single walk that takes the first event at every step, which costs at most the state limit on either path. Either way the result is marked sampled rather than exhaustive. The maximal parallel schedule may take as many rounds as the state limit allows states, so a correct sequential program is never rejected for the length of its schedule. A search that runs against a deadline, the `--duration` of a session or the time of an exhaustive or guided search, carries it into every evaluation: the machine checks it at each explored state and walk step, the kernel path between examples, and an evaluation past it counts as incorrect while the search that set it counts its work as unfinished.

Three differential checks bind the evaluators to the runtime. On 600 random flat programs the machine and the kernel agree on state counts and terminal observations; every state the machine reaches without sharing is reachable under Prism; and the runtime's direct path ends in one of the machine's terminals. These checks found and fixed a symmetry reduction that treated shared introductions as interchangeable across a joint match.

## Objective

For a correct program `P` on examples `E`,

```
J(P) = mean over E of (span + work / p) + λ · size(P)
```

`work` counts events and `span` is the longest chain of causally dependent events, where an event depends on the events that produced the coherences it selects. `span + work / p` is Brent's bound on time with `p` processors (default 4). `size` counts every vertex and edge of the program graph: distinct atoms, rule nodes at every depth, particles, and occurrences. `λ` (default 0.05) trades description length against time; the description term is the minimum-description-length prior that prefers general programs over example tables.

Each task may carry its own goal, the processors `p` and the size weight `λ`; a task without one takes the session's `--processor` and `--size`, and `solve`, which has no session, takes the defaults. The flags and the learner's state files refuse a `p` that is not positive and finite and a `λ` that is negative or not finite, which would make time infinite or negative or reward larger programs. The goal enters the network as two fields of the task's summary token, `log₂ p` and `log₂ λ` rounded, so one network can learn how the best program moves with the goal, and every search, lesson and proof evaluates a task under its own goal. Half of the synthetic tasks draw their goal from `p` in {1, 4, 64} and `λ` in {0.0125, 0.05, 0.2}. `solve --task NAME --processor P --size W` solves a copy of a task under another goal, and `--processor` and `--size` with `--input` define a task together with its goal; either flag alone keeps the other part of the task's own goal, or the default.

Dividing size by concurrency (`work / span`) would reward adding independent busywork, because extra parallel events raise concurrency without doing anything useful. Brent's bound never decreases when work is added, so it cannot be gamed that way; with `p` large it approaches span, and with `p = 1` it is total work.

Planning uses the potential

```
Φ(P) = 2 + clamp((J_baseline − J(P)) / J_baseline, −0.99, 1)     when P is correct
Φ(P) = mean over E of (1 − distance)                              otherwise
```

where `J_baseline` is the cost of the task's reference program, and distance is the normalized minimum matching cost between the produced and expected coherences. A task defined by tests alone has no reference; its baseline is the cost of memorizing its tests, a table with one rule per test whose input differs from its output, which is the description length that any real program should beat. Every correct program outranks every incorrect one, and incorrect programs are still ranked by how close they come. Rewards are differences of `Φ`, so the return of an episode is `Φ(final) − Φ(start)`.

## Edits

An action names a rule node by its depth-first index, so every edit applies equally to top-level rules, rule values nested in particles, and rules inside bodies.

| Action | Effect |
| --- | --- |
| `Insert`, `Remove` | Add or remove an atom occurrence, an edge of the program graph. |
| `Open`, `Close` | Add an empty particle or remove a particle. |
| `Create`, `Duplicate`, `Delete` | Add a rule `[a] ()`, copy a rule in place, or remove a rule. |
| `Nest` | Insert a rule value `[a] ()` into a particle. |
| `Enclose` | Give an output a body containing `[a] ()`. |
| `Detach` | Remove every particle that mentions an atom from both sides of a rule, releasing the rule from a control token so it fires whenever its data is ready. |
| `Analogy` | Apply `Delete`, `Detach`, `Close` or `Remove` to every rule that the chosen rule maps onto under a renaming of atoms, so one decision changes a whole family of rules at any size. |
| `Stop` | End the episode. |

Hidden atoms extend each task's vocabulary so the planner can introduce new intermediate symbols. Nesting depth defaults to the depth of the task's reference program; `--nesting` allows deeper structure.

## Planning and training

The planner follows Gumbel MuZero's policy improvement: Gumbel-Top-k sampling and sequential halving at the root, completed Q-values mixed with the network's value, and deterministic interior selection. Edits are deterministic, so the planner never predicts what an edit produces; it only predicts whether checking the result exactly is worth its cost, as described below. The improved root policy and the Monte Carlo return are the training targets.

A decision that turns a program into a cheaper correct one is also stored four more times with the chosen edit as its whole policy target, so the network learns from the improvements themselves rather than only from the search's improved policy.

Every best known program also becomes a lesson. Starting from the empty program, the learner applies at each step the legal edit that brings the program closest to the solution, measured by the cheapest matching of rules and particles and the atom edits that remain, and ends with `Stop`. Each step becomes a training example with that edit as its policy target and the rest of the gain as its value, and each consecutive pair becomes an example for the judge. Lessons cover solutions of up to eight rules, are rebuilt when a better program is found, and return to the replay buffer every `--teach` seconds under fresh identity permutations; `--teach 0` turns them off.

Each self-play thread steps its games in lockstep so leaf evaluations batch through the network. Evaluations are cached per thread by exact program. Every program the search evaluates, not only the one it ends on, is offered to the archive; a new best must also pass the task's held-out examples, which are checked with the verifying limits. Each thread takes the costs and correctness a candidate must beat from the archive before every decision and remembers the programs the held-out examples rejected, so a thread checks each of them once, and each holds back later candidates for one decision at most.

Every session, and every solve for the tasks it solves, measures each task's reference and archived programs again against its current definition and goal with the ordinary limits, so their costs compare with those of every program the searches find; a correct one must also be correct under the verifying limits and pass the held-out examples, or it is dropped. A program that names an atom outside the task's vocabulary is dropped too, and one no longer correct cannot stay the best, so a task redefined under the same name keeps only what holds for its new definition. A proof mark records the goal it was proven under and whether it covers every program the learner searches or flat programs only, and survives while that goal and the program's cost under it are unchanged; a later proof that covers more replaces it.

Episodes start from an empty program, the reference, the best known program, a perturbation of it, or the most nearly correct incorrect program, so partial progress is resumed across episodes; a task defined by tests alone starts from its best known program or an empty one. The pool holds the curated tasks, label-free synthetic tasks generated from random programs, every program imported by `optimize` and every task defined by `solve --input`; each `train` run adds `--fresh` new synthetic tasks.

## Inferring instead of checking

Most edits make a program worse, and each exact check explores every interleaving of every example. A second output of the network, the judge, predicts the potential of an edited program from that program and its parent's exact evaluation: the median and the 98th percentile, each fitted with the quantile loss. Quantiles suit the potential, which falls in two clusters, below 1 for incorrect programs and from 1 to 3 for correct ones, where a mistake about correctness is an error of one or two rather than a small Gaussian one. A random 15% of fresh exact checks become the judge's training examples.

A new candidate is checked exactly unless its 98th percentile falls below its parent's potential minus 0.02, that is, unless the judge is nearly sure it is worse. An unchecked candidate enters the search with its median as its potential and is checked as soon as the search returns to it. Every move the planner commits to is checked, and only checked programs reach the archive, so a wrong judgment can waste search but never admit an incorrect program.

The judge earns the right to skip. Until it does, it only probes: one new candidate in ten is judged and then checked anyway. It becomes trusted once it has seen at least 256 candidates that were worth checking, meaning no worse than their parent, and would have skipped fewer than 5% of them. While trusted, one skipped candidate in twenty is checked anyway and counted twenty times, so the share of lost candidates stays an unbiased estimate, and trust is withdrawn as soon as that share passes 5%. `--infer` sets that share; `--infer 0` checks every candidate.

## Solving and proving optimality

The size term makes optimality decidable for every task. A correct program spends at least one event on each example whose input differs from its expected output, so its time is at least `T`, the share of such examples times `1 + 1/p`, and its cost is at least `T + λ · size`. Every program larger than `s` therefore costs at least `T + λ · (s + 1)`, and over a finite vocabulary finitely many flat programs have any given size.

`solve` evaluates every flat program of size 0, then of size 1, and so on, with the learner's objective, except rules that consume nothing, which never stop firing. Once `T + λ · (s + 1)` exceeds the cheapest correct program found, or the best known program's cost, no larger program can be cheaper, and the result is proven optimal over flat programs on the task's vocabulary. That is the whole space the learner searches for a task whose reference is flat when rules may not nest; where the learner could also build nested programs, through a nested reference or `--nesting`, the archive records the proof as covering flat programs only, and the command says optimal among flat programs. Searching one size further collects every program that ties it. A size limit or a time budget can end the search first; it then reports the best program found, including those of a size the time budget cut short, and the least cost any program above the last complete size could have. The time budget bounds each evaluation and is checked again before the rules of each atom set are built, and a size whose search it cut short counts as incomplete. Nothing in the search uses a reference program, so `--blind` solves from the tests alone, and the programs it finds are offered to the archive like any discovery.

The enumeration is organized by the exact set of atoms a program uses. Each distinct atom costs one unit of size, so a program of size `s` over `d` atoms has `s − d` left for its rules, and its rules are built from those `d` atoms alone. Atoms that no example mentions are not treated as interchangeable, because the schedule that measures work and span takes events in an order that depends on atom identities.

Two sound filters skip evaluations. A rule whose inputs need an atom that no test input holds and no firing rule can produce never fires; removing it leaves the program's behavior unchanged and makes it smaller, so a program containing it cannot be optimal. Rule values in an input coherence are live rules that can fire and feed the program, which this filter does not model, so it is off for any task whose inputs hold rule values. And many programs grow without end, where a full evaluation can take half a second of state identity computations. Before that, each example replays the one deterministic walk that the objective itself falls back on when exploration overflows, taking the first event at every step. If that walk ends in a wrong observation, outgrows the limits or runs past the state limit, the objective's verdict is incorrect whatever else it explores, so the program is rejected without computing a single state identity.

A proof is exact for the objective as the learner defines it, including its exploration limits and its schedule. The learned search and exhaustive search compose into an anytime optimizer: the learned search lowers the best known cost, each reduction shrinks the space the proof must cover, and the exhaustive search either proves the learned program optimal or hands it a cheaper one.

When exhaustive search ends without a proof, or cannot take the task, `--guide` spends the given seconds on Levin tree search with the network's policy; until a network has been trained and saved in the learner home, it says so and is skipped. It expands programs in order of path length divided by the probability the policy gives the path, mixed with 5% uniform probability over each program's legal edits. Each expansion evaluates 64 programs in parallel, scores their edits in one batch on the GPU and keeps only each program's 24 likeliest edits, so an edit outside them is never tried from that program. Among the kept edits, a path the policy follows with probability `π` to depth `d` is expanded after about `d / π` others, so a better policy makes every search cheaper. A flat program it finds at or below the cost that every unexamined program must exceed is optimal among flat programs, because the exhaustive search already covered every smaller flat program; a nested program it finds proves nothing. For a task exhaustive search could not take, that cost is `T`, because a correct program's time alone reaches it and its size term is not negative.

## Learning loops

`--race N` plays games in pairs from the same task and start. Once both players have stopped, the one whose program has the higher potential wins, fewer edits break a tie, and the winner's every decision is stored `N` more times with its chosen edit as the policy target, so preparatory moves that led to the better program are credited too.

`improve` runs the whole loop for `--round` rounds. Each round generates `--fresh` new behaviors defined by tests alone, half of them with random goals, solves them with `--enumerate` seconds of exhaustive and `--guide` seconds of guided search each before any training on them, under the session's goal and nesting, and then trains with lessons for `--practice` seconds, focused on every usable task still unsolved. Each round's behaviors come from a seed mixed from `--seed`, the round and the pool's size, so rounds draw different behaviors. The share of new behaviors solved in each round, before the network has seen them, measures how much the loop has improved itself. `--duration` bounds the whole run: no round starts after it, and every search and lesson ends with it.

`curriculum` teaches in levels of increasing complexity with one network, up to level `--level`. Level `L` holds behaviors generated from hidden programs of `L` rules, defined by their tests alone. Each level keeps an exam of held-out behaviors whose optimum exhaustive search has proven within `--enumerate` seconds; each exam keeps the goal its optimum was proven under, and they never enter the pool, so neither lessons nor self-play see them. Each round solves the level's `--fresh` training behaviors, trains with lessons for `--practice` seconds on them and on a `--rehearse` share of earlier levels, and then sits every exam so far: guided search may examine `--expansion` programs per behavior and passes when it reaches the proven optimum. A level is mastered when its pass rate reaches `--mastery`, and the next level begins; the earlier exams, sat again every round, show whether anything learned is being lost. Progress is kept in `curriculum.json`. `--duration` bounds the whole run, down to each search, lesson and exam, and a round it cuts short records no mark.

## Hardware

`--device auto`, the default, runs the network on the GPU when Metal is present. One thread serves batched inference to every self-play thread, one thread trains, and two self-play threads share each remaining core, so one computes while the other waits for its batch: on an 18-core M5 Max, 34 threads of 8 games each. `--device cpu` runs inference on the self-play threads and training on `--trainer` threads.

The [gpu](../gpu/) package loads Metal and Metal Performance Shaders at run time through the Objective-C runtime, so the hermetic build needs no Metal SDK. Its only unsafe code is that binding, in `gpu/runtime.rs`; the rest of the package denies unsafe code, and every other crate the learner uses forbids it. Dense layers use `MPSMatrixMultiplication`, and embedding, normalization, attention, the pointer heads and AdamW are Metal kernels. Attention stages key and value tiles in threadgroup memory, computes softmax online, and recomputes probabilities in the backward pass from saved log-sum-exp values instead of storing them. Kernels index memory by the batch's features and pointers, so the engine refuses a batch whose features do not fill whole tokens or exceed their field's values, or whose pointers name a head or token that does not exist, and optimizer state of the wrong length. Tests hold the GPU's outputs, gradient and AdamW update to the CPU implementation. Other platforms build a stand-in engine that reports Metal as unavailable, so the learner falls back to the CPU.

The default network is 256 wide with 6 blocks of eight 32-wide attention heads and a 1,024-wide feed-forward layer, about 5 million parameters. Heads stay 32 wide because each attention thread keeps its query and accumulator in registers. `--width`, `--depth` and `--hidden` choose another shape. A saved network narrower or shallower than the configured shape grows to it without forgetting: each doubled width splits a weight between duplicated coordinates with opposite noise, so the function is unchanged while the copies can diverge, and new blocks and feed-forward units start with zero output. New input fields and new values of existing fields start with zero embeddings, so an encoding can gain inputs, such as the goal, without disturbing what the network knows. Loading a checkpoint checks its header before building anything: the input fields, width and heads, and a parameter count computed in closed form that must match the file.

## Results

Measured on an M5 Max (18 cores, 40-core GPU) on 2026-09-23.

The first run trained a half-million-parameter network for 44 minutes on the CPU over 73 tasks, 25 curated and 48 synthetic: 30,805 episodes and 12,677 training steps. Across its first and last ten minutes, the mean episode return rose from −0.14 to +0.09 and the share of episodes ending in a correct program from 50% to 66%. 45 tasks now have a program cheaper than their reference, each exhaustively correct on its examples and passing its held-out ones:

| Task | Reference | Best | Saving |
| --- | --- | --- | --- |
| `gather.3x3` | 6.000 | 1.650 | 72% |
| `addition.3` | 3.200 | 1.650 | 48% |
| `boolean.parity` | 3.000 | 2.000 | 33% |
| `boolean.and`, `or`, `xor`, `equal` | 2.450 | 2.000 | 18% |
| `boolean.majority` | 3.000 | 2.500 | 17% |
| `maximum.3x3`, `minimum.3x3` | 5.700 | 5.175, 5.225 | 9%, 8% |
| 34 synthetic tasks | | | median 26% |

`addition.3` became the single rule `[Left, Right] ()`, which consumes both operand markers and merges the two unary operands into one coherence.

The trained network carries the search. With learning frozen and the same tasks, archive and search budget for 210 seconds, the trained network's episodes returned +0.16 to +0.20 and ended correct 71% to 79% of the time, and found 6 programs better than any known; the untrained network's returned −0.30 to −0.39, ended correct 38% to 45% of the time, and found none. The trained policy also stops sooner, taking about 7.5 edits per episode against 20.5.

Moving the network to the GPU speeds the whole loop, because self-play is bound by exact evaluation on the CPU cores. Each configuration ran 210 seconds from the same learner:

| Configuration | Self-play threads | Evaluations | Samples | Training steps |
| --- | --- | --- | --- | --- |
| CPU, 0.5M parameters | 14 | 224,284 | 16,648 | 1,261 |
| GPU, 0.5M parameters | 17 | 360,789 | 27,709 | 2,004 |
| GPU, grown to 5M parameters | 17 | 208,567 | 14,851 | 929 |
| GPU, grown to 5M parameters | 34 | 299,572 | 20,512 | 1,279 |

Training ran at the replay ratio cap in every configuration. The larger network makes each inference batch slower, and two self-play threads per core hide most of that wait. A GPU training step on 128 samples of about 120 tokens takes 20 ms for the half-million-parameter network and 89 ms for the 5-million-parameter one, 2.7 and 4.8 times faster than 16 CPU threads.

### From gnome sort to a parallel sort

Gnome sort is in the pool at seven sizes. A `Gnome` token walks the positions: it compares its two neighbours, swaps them and steps back, or steps forward. Every step waits for the one before it, so work and span are equal. Tasks up to 32 inputs list every input; larger ones keep 24 or 27 as examples and hold the rest out, so only a program that sorts every input can become a task's best.

Without `Detach` the planner found 11% to 22% savings in 17 minutes, all of them input-domain shortcuts: with only the values 0 and 1, a gnome that sees two 0s behind it may stop early. It never left the sequential walk. Removing the gnome from a swap rule takes two edits, one on each side of the rule, and the program is wrong between them. `Detach` makes that one edit, and each such edit keeps the program correct while lowering its cost. A test climbs from gnome sort to the parallel exchange sort taking the best correct improving edit at every step.

After 30 minutes with `Detach`, spending half of the episodes on these tasks, measured on every input:

| Task | Reference | Best | Saving | Work | Span | Exchange sort |
| --- | --- | --- | --- | --- | --- | --- |
| `gnome.4x2` | 22.350 | 4.812 | 78.5% | 8.00 → 2.50 | 8.00 → 1.69 | 4.763 |
| `gnome.3x3` | 25.550 | 16.994 | 33.5% | 6.00 → 4.96 | 6.00 → 3.70 | 6.270 |
| `gnome.5x2` | 29.950 | 20.469 | 31.7% | 11.00 → 9.00 | 11.00 → 7.72 | 6.275 |
| `gnome.4x3` | 37.900 | 27.952 | 26.2% | 9.00 → 7.15 | 9.00 → 6.77 | 9.101 |
| `gnome.6x2` | 38.175 | 31.195 | 18.3% | 14.50 → 13.52 | 14.50 → 13.52 | 7.912 |
| `gnome.5x3` | 51.083 | 43.677 | 14.5% | 12.67 → 11.79 | 12.67 → 11.68 | 12.125 |
| `gnome.7x2` | 47.025 | 43.003 | 8.6% | 18.50 → 17.56 | 18.50 → 17.56 | 9.644 |

On four elements the learner reached the parallel exchange sort by itself: the gnome is discarded and every adjacent pair exchanges whenever it is out of order.

```
[Gnome.First],
[First.1, Second.0] (First.0, Second.1),
[Second.1, 0.Third] (Second.0, Third.1),
[0.Fourth, Third.1] (0.Third, 1.Fourth)
```

The larger sizes were still converting when the run ended. The runtime's kernel and Prism both confirm every best program on every input, held-out ones included.

The program found on four elements is one instance of a rule family that holds at every length `n` and every value range: discard the gnome, and for every adjacent pair of positions and every pair of values `h > l`, exchange `[P.h, Q.l] (P.l, Q.h)`. Every rule rewrites whole element coherences in place, so only values move. An exchange fires only on an adjacent pair that is out of order and puts it in order, which lowers the number of inversions `I` by exactly one, so every execution performs exactly `I` exchanges and stops. When no exchange applies, no adjacent pair is out of order, so the values are sorted, and a multiset has one sorted arrangement: every schedule ends in the same observation. `I + 1` events is the least work any program that moves values by adjacent exchanges can do; gnome sort takes `1 + n + 2I` events, one after another. Instantiated beyond every trained size and checked exhaustively:

| Length | Values | Inputs | Gnome work and span | Exchange work | Exchange span | Cost |
| --- | --- | --- | --- | --- | --- | --- |
| 6 | 2 | 64 | 14.50 | 4.75 | 2.88 | 38.175 → 7.963 |
| 8 | 2 | 256 | 23.00 | 8.00 | 4.20 | 56.500 → 11.495 |
| 10 | 2 | 1,024 | 33.50 | 12.25 | 5.52 | 77.325 → 15.279 |
| 6 | 3 | 729 | 17.00 | 6.00 | 3.37 | 65.100 → 15.324 |
| 4 | 4 | 256 | 9.50 | 3.25 | 2.09 | 58.525 → 15.248 |

Positions in this encoding are atoms, and a flat rule names the atoms it matches while the remainder of a rule is copied to every output, so no single flat program can exchange neighbours in a sequence of any length. A sort over every length needs structure made of first-class rules, like the linked vectors below; within flat programs the generality lives in the rule family.

With `Analogy`, one decision releases every swap rule at once, whatever its positions and values are called, so the conversion is about five improving decisions at every size. Twenty minutes with self-imitation, spending half of the episodes on all seven sizes together:

| Task | Reference | Best | Saving | Best time | Exchange sort |
| --- | --- | --- | --- | --- | --- |
| `gnome.4x2` | 22.350 | 4.763 | 78.7% | 2.312 | 4.763 |
| `gnome.6x2` | 40.467 | 8.708 | 78.5% | 4.708 | 7.912 |
| `gnome.3x3` | 25.550 | 6.520 | 74.5% | 1.870 | 6.270 |
| `gnome.5x2` | 29.950 | 9.775 | 67.4% | 3.125 | 6.275 |
| `gnome.4x3` | 37.067 | 14.694 | 60.4% | 2.444 | 9.101 |
| `gnome.7x2` | 46.713 | 36.019 | 22.9% | 17.219 | 9.644 |
| `gnome.5x3` | 51.176 | 45.693 | 10.7% | 14.093 | 12.125 |

Costs here are measured on each task's examples. Without `Analogy`, 30 minutes left six elements at 17%; with it, six elements became the exchange sort with one redundant copy of the gnome's discard rule. The two largest sizes were still converting when the run ended. The runtime's kernel and Prism confirm every best program on every input.

For the general case, the objective itself ranks the three ways to sort. Measured exhaustively:

| Length | Values | Gnome sort | Lookup table | Exchange sort |
| --- | --- | --- | --- | --- |
| 3 | 2 | 15.375 | 10.350 | 3.438 |
| 5 | 2 | 29.950 | 56.050 | 6.275 |
| 7 | 2 | 47.025 | 296.150 | 9.644 |
| 4 | 3 | 37.900 | 115.050 | 9.101 |
| 5 | 3 | 51.083 | 414.800 | 12.125 |

A lookup table finishes in one event, but it needs one rule per input, so its size grows as the number of values raised to the length. A smaller size weight lets a table win the smallest cases; for any positive weight, the table loses beyond some finite length, while the exchange sort's size grows linearly and it beats gnome sort in work, span and size at every length.

Without self-imitation the network did not learn this structure: shown gnome sort at a size it never trained on, six elements of three values, it spread its first-move probability across the kinds of edit in proportion to their number, before and after the run above.

With self-imitation, spending half of 25 minutes on only the three smallest sizes, the learner reached the exchange sort on all three within 17 minutes: `gnome.4x2` at 4.763 and `gnome.5x2` at 6.275, both exactly the exchange sort's cost, and `gnome.3x3` at 6.320 against 6.270. On the unseen six-element gnome sort the trained network now gives detaching a rule 3.6 times its uniform share instead of 1.2, closing a particle 2.4 times, and inserting an atom 0.7 times; it does not yet tell the swap rules, where detaching is right, from the forward rules, where it breaks the program. Frozen for ten minutes on the three largest sizes, starting from the same archive, the trained network went further than the untrained one on every task:

| Task | Start | Trained network | Untrained network |
| --- | --- | --- | --- |
| `gnome.6x2` | 36.483 | 32.408 | 33.483 |
| `gnome.7x2` | 44.379 | 40.829 | 42.779 |
| `gnome.5x3` | 48.213 | 42.115 | 46.652 |

### Inferring instead of checking

Starting from the network trained with `Analogy`, with the gnome tasks reset to their references, the judge trained for ten minutes. Its median missed the exact potential of programs it had not seen by 0.82 at first and by 0.12 to 0.18 after 90 seconds, and it earned trust after four minutes. From that network, two ten-minute runs spent half of their episodes on the seven gnome sizes, one checking every candidate and one letting the judge skip:

| | Every candidate checked | Judge may skip |
| --- | --- | --- |
| Decisions | 9,176 | 9,720 |
| Exact checks per decision | 13.55 | 13.24 |
| Network inferences per decision | 13.40 | 19.53 |
| Candidates judged | | 64,006 |
| Candidates skipped | | 2,742 |

The judge earned trust again within a minute, lost between 0% and 7% of the worthwhile candidates in each 30-second window, and twice lost its trust and earned it back. It was nearly sure that only one judged candidate in 23 was worse than its parent, so exact checks per decision fell by 2%, while judging a candidate before checking it raised inference by 46%, which the GPU absorbed. The judged run went further on `gnome.5x2`, `gnome.4x3` and `gnome.5x3` and less far on the other four sizes, within the spread of single runs.

Two things hold it back. The policy already steers the search away from edits that are obviously worse, so the candidates the search reaches are those whose effect is uncertain until they run. And the judge sees the edited program but not the edit. A judge that scores every edit from the parent's representation, as the policy does, would see exactly what changed and take one inference for all of a program's children instead of one per child; counting how often each rule fired in the parent's execution would tell it which edits touch live rules.

Two pilots shaped the design. A Gaussian judge was overconfident, because a mistake about correctness is not a small error: 13% to 19% of the candidates it would have skipped were no worse than their parent. Measuring that share among skipped candidates also made trust depend on how many candidates were skipped, so the gate now measures the share of worthwhile candidates that would be lost.

### Proven optimal programs

The first version of this search ran on the 25 of the 80 tasks whose spaces a generating-function count put under sixty million programs, before dead rules were skipped, which later cut the boolean proofs from about 49 to 38 seconds. Every best program the learner had found is optimal, and on every one of these tasks it is the only flat program over the vocabulary that reaches its cost:

| Task | Reference | Optimum | Size bound | Programs enumerated | Seconds |
| --- | --- | --- | --- | --- | --- |
| `boolean.not` | 2.000 | 2.000 | 15 | 9,965,443 | 49 |
| `boolean.and`, `or`, `xor`, `equal` | 2.450 | 2.000 | 15 | 9,965,443 each | 48 to 55 |
| `boolean.parity` | 3.000 | 2.000 | 15 | 9,965,443 | 48 |
| `addition.3` | 3.200 | 1.650 | 8 | 2,348 | 0.0 |
| `gather.3x3` | 6.000 | 1.650 | 8 | 2,348 | 0.1 |
| 17 synthetic tasks | | 0% to 50% below reference | 5 to 14 | 58 to 9,465,665 | 0 to 68 |

The optima answer with the inputs a rule leaves untouched. `[Xor.False] ()` removes the operator and one `False`, and the remaining value is the answer, because false xor x is x; only true xor true needs a rule of its own, `[Xor.True.True] (False)`. Parity over three values cancels a matching pair and keeps the third, with `[Parity.True.True] ()` and `[Parity.False.False] ()`. Three synthetic references were already optimal.

The other tasks are beyond enumeration: `boolean.nand` and `boolean.nor`, whose best is still the reference, would need every program up to size 24, `boolean.majority` up to 25, and the gnome sorts up to sizes 141 to 573.

### Solving from tests alone

Two hundred behaviors were generated from random hidden programs, each defined by ten tests with six more held out, and the programs were removed before solving. `solve --blind` gave each task five seconds of exhaustive search; then one twenty-minute `train` run, from the same network, spent half of its episodes on the tasks still unsolved:

| Stage | Solved on its tests | Passing held-out tests | Proven optimal | Time |
| --- | --- | --- | --- | --- |
| Exhaustive search, five seconds each | 96 | 83 | 58 | 13 minutes |
| Learned search on the other 104 | 55 | 55 | | 20 minutes |

With two overfitted tasks the learned search later solved properly, 140 of the 200 behaviors hold a program that passes its held-out tests, found from tests alone in 33 minutes. Against the hidden programs that generated the tests, 100 of those programs are cheaper and 40 cost the same; none costs more. Exhaustive search reached about size 13 in five seconds, and the learned search's solutions have sizes 12 to 19, median 14. In 13 tasks the cheapest program consistent with ten tests fails the held-out ones: the tests leave the behavior open, and a shortest program is only as general as its tests demand.

### Learning from solutions

From the home of the batch above, with 49 of its behaviors still unsolved and 100 fresh ones defined by tests alone, two fifteen-minute runs of the same network spent half of their episodes on those 149 tasks; one learned from lessons of the 315 best known programs it could demonstrate:

| Run | Solved | Of the 49 hard | Of the 100 fresh | After two minutes |
| --- | --- | --- | --- | --- |
| Without lessons | 72 | 10 | 62 | 50 |
| With lessons | 102 | 22 | 80 | 71 |

No lesson showed a program for any of these tasks, so the gain is transfer: learning how other programs are built made the search solve 42% more new tasks, more than twice as many of the hard ones, and solve them sooner.

### Guided search

The network trained with lessons then met 60 new behaviors defined by tests alone, with five seconds of exhaustive search and ten of guided search each. Exhaustive search found programs for 30 and guided search for 33, 18 of them ones exhaustive search had missed, at sizes 12 to 19. Together they found 48, of which 42 pass their held-out tests and 29 are cheaper than the hidden program that produced the tests. Guided search needed a median of 202 programs to reach its first correct one, where exhaustive search examined 350,000 to 660,000 programs in its five seconds.

### Improving itself

Four rounds of `improve` from the network trained with lessons each generated 40 new behaviors, solved them before training with three seconds of exhaustive and six of guided search each, and then trained for six minutes:

| Round | Found | Passing held-out tests | Proven optimal |
| --- | --- | --- | --- |
| 1 | 34 | 28 | 14 |
| 2 | 29 | 24 | 10 |
| 3 | 32 | 28 | 16 |
| 4 | 37 | 31 | 12 |

The last round was the best, but every round draws a different batch of 40, so the rise is within the spread of a single round; fixed exams, as in `curriculum`, measure progress more tightly.

### Curriculum

From the network trained by `improve`, the curriculum graded level 1 with 20 held-out behaviors of one rule whose optima it had proven, solved 40 more for training, and trained for five minutes. Guided search then reached the proven optimum of every exam behavior within 256 programs, so level 1 was mastered after a single round. The run was paused during the first round of level 2: carrying one network through levels whose programs outgrow exhaustive proof needs far more training than one machine supplies in hours.

### Racing

With lessons on, fifteen minutes of racing (`--race 1`) on the same 149 tasks solved 102, 23 of the hard ones and 79 of the fresh ones, the same as lessons alone, and trailed early, 74 against 84 after five minutes. From one start the two players often finish level, and a win by a small margin is often luck, so the winner's decisions add little to what lessons and the improving decisions already teach. Racing stays available but off.

### Goals

Thirty generated behaviors that the default goal had been proven optimal on were solved again under three other goals with ten seconds each. Every one was solved under every goal, and proofs held for 13 with one processor, 30 with `λ = 0.2` and 27 with 64 processors and `λ = 0.0125`. The optimal program changed with the goal once: `synthetic.76` is optimal as `[Charlie], [Echo]` with four processors, but with one processor, where all work costs time, a third rule `[(), Charlie]` that clears two coherences in one event pays for its four extra units of size by cutting the work per example from 1.10 to 0.90. Behaviors this small rarely hold a trade-off between time and size, so their optimum rarely depends on the goal; larger tasks do, such as sorting, where a lookup table is optimal when size costs little and an exchange sort otherwise.

### The linked vector library

The learner executes `program/vector/sort.wave` with its whole library, merge sort, merge, reverse, nodes, natural comparison, digits and chains: 668 rules counting nested rule values, 176 atoms. Sorting six numbers takes 2,170 events of work with a span of 1,685, so 78% of the work lies on one causal chain; the handshakes between sealed nodes and the digit-by-digit comparison order almost everything.

It cannot yet optimize the library. Its atoms exceed the encoding's 128 identities. A built number or vector is a frame of sealed live rules rather than a configuration, so the library's functions cannot be given inputs directly; a program has to build its own data, which leaves one example that a hard-coded answer would satisfy. Each evaluation takes about 37 seconds.

## Scaling

The learner grows by turning what it has found into larger units of reasoning and by teaching itself from every solution, so each solved program makes the next one cheaper. In place:

- Edits by analogy change a whole structural family of rules in one decision, so the number of decisions no longer grows with the program.
- Lessons turn every best known program into demonstrations, which solved 42% more unseen tasks in the same time.
- Guided search spends its effort in the order its policy prefers among each program's likeliest edits, and reaches programs far past exhaustive search.
- Goals make the objective an input, so one network can serve any balance of time, parallelism and size.
- The curriculum grades behaviors by complexity, examines each level against proven optima and rehearses earlier levels.

Next:

1. Compute: the loops are ready to run for weeks on many machines, with self-play spread across them and training shared; the measured gains so far came from minutes on one laptop.
2. Library learning: compress edit sequences that repeatedly improve programs into named transformations, and recurring rule families into schemas, and let the network choose among them.
3. Generic verification: a flat program is correct under every schedule when it terminates and its rules' overlaps rejoin; checking the overlaps and finding a decreasing measure proves a rule family for every input size, instead of enumerating inputs.
4. Rewrites at scale: apply every proven improving rewrite wherever it matches, keeping learned search for discovering new rewrites.
5. Modules: split large programs at their protocol boundaries, optimize each module against its observed behavior, and recompose.
6. A judge of edits: predict each edit's effect from the parent's representation and from how often each rule fired, so one inference covers every child and exact checks go only to edits whose outcome is uncertain.
7. Lower bounds beyond enumeration: bound a task's least possible work from the information its outputs need, and prove families of programs optimal, so proofs reach sizes that no enumeration can.
8. Throughput: guided search examines hundreds to a few thousand programs a second, bound by exact evaluation and batch size, and the exhaustive search spends tens of microseconds per candidate; both have room to grow severalfold.

## Limits

A task specifies behavior only on its examples, so an optimized program is guaranteed correct on those inputs under every schedule, and only probed on held-out ones. Programs whose state spaces exceed the exhaustive budget are decided by sampled walks. A state with more than 1,024 rule matches (65,536 when verifying) exceeds the limits; such a program is too nondeterministic to decide and counts as incorrect. On the kernel path the same numbers bound the dispatch network's steps at each state; each step yields at most one match, so that bound is at least as strict. The kernel path rebuilds the dispatch network at every state, which keeps it simple and exact but makes nested programs far slower to evaluate than flat ones. Imported programs larger than the encoding's identity spaces share identities, except atoms: `optimize` refuses a program naming more than 124 atoms, which with the four hidden atoms would exceed the encoding's 128 identities. The network has begun to prefer the structural edits that transfer across sizes, but on large programs most progress still comes from search, so convergence slows as programs grow. The judge skips few checks so far, because the candidates the search reaches are rarely ones it can dismiss without running them. Racing added nothing measurable, and a goal changes the optimum only where a behavior holds a real trade-off between time and size. A proof of optimality covers flat programs over a task's vocabulary and is exact for the objective, limits and schedule included; it says nothing about nested programs, which is why proofs for tasks that admit them are recorded as covering flat programs only; the number of candidates grows three- to fourfold with each unit of size, so ten million programs up to size 15 take under a minute, while the gnome sorts would need sizes in the hundreds.
