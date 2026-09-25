# Spectrum

Spectrum answers questions about a Photonic program and every configuration it reaches: what can happen, whether a claim holds, why a configuration or an occurrence is there, why a target is not, and what an edit changed. Each question is a typed request with a typed answer. The same questions reach it as `photonic` commands that print text or JSON, as JSON requests to the [spectrum](../spectrum/) library, and as tools over the Model Context Protocol. Answers name what they found by handles that later questions accept, so a question can move from an overview to one occurrence without reading the whole exploration.

| Package | Responsibility |
| --- | --- |
| [`//spectrum`](../spectrum/) | Requests and answers, explorations, handles, patterns, claims, lineage and the store. It reads the runtime through read-only accessors and orders programs with the [symmetry](symmetry.md) engine. A `Reader` supplies file contents, so the library touches no file system. |
| [`//command:photonic`](../command/) | One command for every tool: the engine's `parse`, `lower`, `run` and `prism`, the verbs with their exit codes, and `mcp`, the protocol server. Every path is read from where `bazel run` started. |

## A session

The conjunction example in [program/language/conjunction.wave](../program/language/conjunction.wave) opens a scope with the conjunction's three cases, and the scope matches what `True` and `False` can become.

```sh
bazel run -c opt //command:photonic -- explore program/language/conjunction.wave
```

```
x91c7f6619ec81b4b · closed · 14 configurations · 17 events, 4 inferred · depth 3 · work 240 · shape 2a2059d40174b9fb
end    s4    in f1: Extra
       s6    in f1: Boolean.Extra
       s12   Boolean.Extra
       s13   in f1: Boolean.Boolean.Extra
rule   r0    [And.Boolean.Boolean] (…)   4, 3 inferred
       r1    [True.True] True            never   in the scope r0 opens
       r2    [True.False] False          1   in the scope r0 opens
       r3    [False.False] False         never   in the scope r0 opens
       r4    [True] Boolean              5
       r5    [False] Boolean             7, 1 inferred
```

The first line is the exploration's key, whether it closed, its size and its shape. `in f1:` places a coherence inside the scope that frame 1 holds. A claim asks whether the program reaches exactly `False.Extra` with its rules, and `cause` shows how:

```sh
bazel run -c opt //command:photonic -- check program/language/conjunction.wave --reach False.Extra --exact --preserve
bazel run -c opt //command:photonic -- cause program/language/conjunction.wave s9
bazel run -c opt //command:photonic -- cause program/language/conjunction.wave s9.o0
```

```
no diagnostics
reach False.Extra exactly   holds   s9 by e9 e10   s9 is the target
x91c7f6619ec81b4b · closed · 14 configurations · 17 events, 4 inferred · depth 3 · work 240 · shape 2a2059d40174b9fb

s9 False.Extra
path       e9    r0 [And.Boolean.Boolean] (…)   s8 in f1: True.False.Extra   inferred
           e10   r2 [True.False] False          s9 False.Extra

False in s9 = False.Extra
  e10   r2 [True.False] False   produces False from s8.o1 True, s8.o2 False, s8.o0 And
```

`e9` is inferred: `r0` matches `And.Boolean.Boolean` in what `s0` becomes after `e0` and `e2`, so `inspect e9` separates the exact part of the match from the witness that the deduction supplied:

```
e9 r0 [And.Boolean.Boolean] ([True.True] True, [True.False] False, [False.False] False) · s0 → s8
deduction  inferred by e0 e2
exact      s0.o0 And
witness    s0.o1 True · s0.o2 False
reads      s0.o4 ([And.Boolean.Boolean] ([True.True] True, [True.False] False, [False.False] False))
```

`miss r1` explains why the case for two trues never fires: wherever the rule is live, its scope holds at most one `True`.

```
r1 [True.True] True   never fires · closed · live in 8 configurations where it does not fire
s8    in f1: True.False.Extra     [True.True] lacks True in s8.c0
s7    in f1: True.Extra           [True.True] lacks True in s7.c0
s11   in f1: Boolean.True.Extra   [True.True] lacks True in s11.c0
```

Suppose an edit shortens the scope's last two rules to one, `[False] False`, and saves the result as `bug.wave`. `compare` shows what the edit changed, handles on the right naming the edited program:

```sh
bazel run -c opt //command:photonic -- compare program/language/conjunction.wave bug.wave
```

```
compare x91c7f6619ec81b4b x89472f7eb450ddf2
configurations   14 → 18
  + Boolean.Boolean.Extra   s17   by e24 e20
  + False.Boolean.Extra     s15   by e11 e12 e16
  + Boolean.True.Extra      s14   by e24
  + False.True.Extra        s11   by e11 e12
events           17 → 25
  + [False] Boolean      False.Boolean.Extra → Boolean.Boolean.Extra
  + [False] Boolean      False.True.Extra → Boolean.True.Extra
  + [False] Boolean      False.Boolean.And.Extra → Boolean.Extra   inferred
  + [False] Boolean      False.And.True.Extra → Boolean.True.Extra   inferred
  + [False] False        in f1: False.Extra → False.Extra
  + [False] False        in f1: False.Boolean.Extra → False.Boolean.Extra
  + [False] False        in f1: False.True.Extra → False.True.Extra
  + [True] Boolean       Boolean.True.Extra → Boolean.Boolean.Extra
  + [True] Boolean       False.True.Extra → False.Boolean.Extra
  − [True.False] False   in f1: True.False.Extra → False.Extra
```

`[False] False` matches `False` alone, so the conjunction's `True` survives it. The lineage of that `True` in `bug.wave` shows where it came from:

```
True in s11 = False.True.Extra
  s0                                   initial in False.And.True.Extra
  e11   r1 [And.Boolean.Boolean] (…)   inferred by e0 e2; the scope receives True as witness
  e12   r2 [False] False               consumes False; True stays in the remainder
```

The claim `--avoid False.True.Extra --exact --preserve` holds on the original and fails on the edit with `s11 by e11 e12`, and `compare` accepts the same claims to report both answers.

## Recordings

Every question except `shape` is about a recording: a program to explore, or the key of an exploration an earlier answer returned. Its fields sit beside the question's own fields, and `compare` takes one recording as `left` and one as `right`.

| Field | Meaning |
| --- | --- |
| `program` | The program: `file`, a list of `.wave` or `.particle` sources or `.json` programs that Bazel assembled, read in order; `source`, Photonic written inline and read after the files; and `library`, files of declarations loaded first. |
| `exploration` | The key of an exploration, such as `x91c7f6619ec81b4b`, instead of `program`. A key already fixes the mode, budget and goal, so it comes alone. |
| `mode` | `exhaustive`, the default, explores every configuration within the budget; `path` follows one direct path, as `photonic_test(path = True)` does. |
| `budget` | The limits below. |
| `goal` | In path mode only, the configuration the path stops at: `{"configuration": "False.Extra", "preserve": true}`, where `preserve` adds every loaded root rule, as Prism reads targets. |

| Budget field | Default | Limits |
| --- | --- | --- |
| `work` | 2,000,000 | Work steps before the search stops. |
| `configuration` | 4,096 | Configurations kept. |
| `coherence` | 64 | Coherences in one configuration. |
| `occurrence` | 256 | Occurrences in one configuration. |
| `scope` | 64 | Scopes in one configuration. |
| `record` | 2,000,000 | Records the engine retains. |

The defaults and the names are `photonic_test`'s. An exploration closes when its search ends within the budget; otherwise it is open, and every answer that depends on what was not explored is unknown. The runtime also records configurations and events that no grounded sequence of events establishes. They keep their handles, and answers mark them unsupported, but claims, paths, lineage, firing counts and comparisons consider only supported ones.

### Canonical order

Before exploring, Spectrum names the program's atoms A, B, C and so on in the order of its shape, the canonical form the symmetry engine computes, and lists its rules and coherences in that form's order. Answers translate the letters back to the program's own names, and each rule's name is its printed text. Configurations, events and rules are numbered in the order this canonical program explores them, so reordering a program's terms, or the parts of a term, keeps every handle, and so does renaming its atoms. When the symmetry search visits more than 100,000 nodes, Spectrum sorts the program's printed rules and coherences instead, which keeps handles under reordering only. A direct path follows the program as written, because its scheduler chooses by source order. Each summary's `order` says which of `shape`, `text` and `source` numbered its handles.

Within a particle, answers list occurrences in canonical order, which is the order of their handles; `bug.wave` prints its first configuration as `False.And.True.Extra`. Two programs can order the same particle differently, so `compare` may print one particle two ways.

### Keys and the store

An exploration's key is `x` followed by 16 hexadecimal digits: a hash of the canonical program, the naming from letters to atoms, the mode, the budget and the goal. In exhaustive mode, programs that differ only in the order of their terms share a key. Programs that differ in their names do not, because their answers name different atoms. The store keeps the most recent explorations, 16 by default and 64 in the protocol server, and accepts any unambiguous prefix of a key of at least four digits. The command line starts a new store for each command, so keys carry across questions in one protocol session or one library context.

## Handles

| Handle | Names |
| --- | --- |
| `r2` | A rule, numbered in canonical order, including rules that scopes open. |
| `s11` | A configuration. `s0` is the start. |
| `e12` | An event: one rule applied to one match, from one configuration to another. |
| `s11.c0` | A coherence of a configuration. |
| `s11.o1` | An occurrence in a configuration, in a coherence or held by a scope. |
| `s10.f1` | A scope frame of a configuration. Frame 0 is the root. |

## Patterns

Patterns are Photonic, matched by containment as a rule's input is: whatever else a coherence holds does not matter.

| Pattern | Selects |
| --- | --- |
| `B` | A coherence holding `B`. |
| `B.X` | A coherence holding both. |
| `B, C` | Two different coherences. |
| `([A] B)` | A coherence holding that rule value; Spectrum reads it as `().([A] B)`. |
| `[B, C] D` | The events that apply a rule with that structure; spacing and the order of unordered parts do not matter. |

A coherence pattern matches configurations in `select`, `miss` and claims; a rule pattern matches events in `select`.

## Claims

A claim is a pattern with a kind. Its answer is `holds`, `fails` or `unknown`, and a definite answer comes with evidence: a witness configuration and a path to it.

| Kind | Holds when | Decided before the exploration closes |
| --- | --- | --- |
| `reach` | Some configuration matches. | Holds with a witness and its shortest path. |
| `avoid` | No configuration matches. | Fails with a witness and its shortest path. |
| `always` | Every configuration matches. | Fails with a counterexample and its shortest path. |
| `inevitable` | Every run reaches a match: no run ends, or cycles forever, without one. | Holds when the start matches; fails with a cycle that avoids every match. A run that ends without a match counts once the exploration closes. The path reported avoids every match. |
| `outcome` | Every end configuration matches. | Never: until the exploration closes, a configuration without events may be unexplored rather than an end. |

`exact` compares whole configurations as Prism does, and `preserve` adds every loaded root rule to the target; only `reach` and `avoid` take an exact target, and only in exhaustive mode. Inference lets a rule apply to what a configuration can become, so a run can skip configurations a longer run passes through. A direct path follows one run of many, so on a path `reach` can hold, `avoid` and `always` can fail, and every other answer is unknown. Unknown means the search could not settle the claim, because a budget stopped it or because a path follows one run. It is never evidence of absence.

## Questions

| Verb | Fields beside the recording | Answer |
| --- | --- | --- |
| `check` | `claim` | Diagnostics, each claim's verdict and the exploration's summary. A program or library that does not parse or lower is a diagnostic with its location, not a failure. Given a key, it checks claims against that exploration. |
| `explore` | `claim`, `limit` (12) | The summary, the configurations without supported events up to `limit`, how often each rule fired and whether by inference, and each claim's verdict. The text calls those configurations ends in a closed exhaustive exploration, leaves in an open one, where some are unexplored, and stops on a path. |
| `select` | `pattern`, `limit` (20), `offset` | The matching configurations with the occurrences each match used, or the matching events, marked when unsupported, and the offset of the next page. |
| `inspect` | `handle` | A rule with its events; a configuration with its coherences, frames and the events into and out of it, and `end` when a closed exhaustive exploration proves no event can happen there; a coherence, occurrence or frame; or an event with its exact part, witness, reads, deduction and the occurrences it produces. |
| `cause` | `handle` | For a configuration, its shortest supported path. For an event, its match and the events of its deduction. For an occurrence, its lineage: back through each event that carried it as remainder, witness or held occurrence, to the start or to the event that produced it, with the places that event consumed. |
| `miss` | `target` with `exact` and `preserve`, or `rule`; `limit` (3) | For a target, the nearest configurations and what each lacks and, when exact, has extra, assigning the target's parts to coherences so that the most occurrences match; an exact target compares coherences, live root rules and open scopes, as Prism does. For a rule, how often it fired, how many configurations it was live in without firing, and where its inputs came closest to matching. |
| `step` | `handle` (`s0`) | Every event that can happen at a configuration and where it leads; on a path, the one event the path took. |
| `compare` | `left` and `right`, each a recording; `claim`; `limit` (12) | Each side's key and size; the configurations one reaches and the other does not, compared by their coherences and held occurrences up to occurrence identity; the events that differ by rule, source and target, which is where edited rules show; and each claim's answer on both. Each change lists up to `limit` entries in each direction and counts the rest. |
| `shape` | `program` (a list of programs), `target`, `fix`, `node` (1,000,000) | For one program, its shape, canonical form, symmetries, orbits and local patterns; for several, the classes that share a shape with the renaming between members, as the [symmetry record](symmetry.md) describes. `node` limits the symmetry search. |

A request to the library is one JSON object with `version` and `verb` beside the question's fields, and the answer comes back in an envelope with `answer` or `error`:

```json
{"version": 1, "verb": "cause", "program": {"file": ["bug.wave"]}, "handle": "s11.o1"}
```

```json
{"version": 1, "verb": "cause", "answer": {"exploration": "x89472f7eb450ddf2", "handle": "s11.o1", "kind": "occurrence", "text": "True", "configuration": "False.True.Extra", "lineage": [{"configuration": "s11", "occurrence": "s11.o1", "event": "e12", "rule": "r2 [False] False", "role": "remainder", "source": ["s10.o2"], "text": "consumes False; True stays in the remainder"}, …]}}
```

Every field is checked against the question's schema, so a misspelled field is an error that lists the fields the question takes, never a silent default.

## Failures

A failure carries a `code`, a `message` and, when it points into text, a `location` with the file, line, column and length.

| Code | Meaning |
| --- | --- |
| `request` | The request is malformed: not an object, an unknown field, no program, a program and a key together, a key with a mode, budget or goal, or a goal outside path mode. |
| `version` | The request's version is not 1. |
| `file` | A file cannot be read. |
| `source` | A program does not parse or lower; `check` reports it as a diagnostic instead. |
| `library` | A library does not parse or holds more than declarations; `check` reports it as a diagnostic instead. |
| `target` | A goal or an exact target does not parse. |
| `pattern` | A pattern is empty or does not parse, mixes coherences with rules, or is the wrong kind for the question. |
| `handle` | A handle is malformed, names nothing in the exploration, or names a kind the verb does not explain. |
| `exploration` | A key is unknown or ambiguous, or lineage is asked of a direct path. |
| `claim` | A claim cannot be asked this way, such as an exact `always`, an exact target on a direct path, or a rule pattern as a claim. |
| `shape` | The symmetry search exceeded its `node` limit. |

The command prints answers as text, or with `--json` as the envelope, and prints a failure as `error[code]: message`. It exits 1 on any failure; `check` also exits 1 unless there are no error diagnostics and every claim holds, `compare` unless both sides reach the same configurations and events and every claim answers alike, and `shape` when its programs have more than one shape. The library decides this through `Answer::passed`.

## Model Context Protocol

`photonic mcp` serves every question as a tool over standard input and output, one JSON-RPC message or batch per line, and the repository's [.mcp.json](../.mcp.json) starts it with `bazel run`. It speaks the stateless revision 2026-07-28, in which every request carries `io.modelcontextprotocol/protocolVersion` and `io.modelcontextprotocol/clientCapabilities` in its `_meta` and `server/discover` lists the supported versions, and the revisions 2025-11-25, 2025-06-18, 2025-03-26 and 2024-11-05, through `initialize` or by naming one of them in `_meta`.

Each tool's input schema is JSON Schema 2020-12 generated from its request type, with every subschema inlined and no properties beyond those listed. From 2025-06-18 on, each tool also declares an output schema, generated for serialization so that fields an answer leaves out are optional, and a result carries the answer as `structuredContent` beside its text. `//spectrum:test` checks every kind of answer against its declared schema. A question that fails returns a result with `isError` and the failure's text, so an agent can read it and ask again; protocol errors are reserved for the protocol itself.

| Code | Error |
| --- | --- |
| -32700 | A line is not JSON. |
| -32600 | A message is not a request object, names no method, or is an empty batch. |
| -32601 | The method is unknown. |
| -32602 | The tool is unknown, a 2026-07-28 request lacks its client capabilities, or, in 2026-07-28, a resource is unknown. |
| -32002 | A resource is unknown, before 2026-07-28. |
| -32022 | The requested version is unsupported; `data` lists the supported versions. |

Two resources describe the language to agents: `photonic://primer`, the grammar, what programs mean and how to ask Spectrum, from [spectrum/primer.md](../spectrum/primer.md), and `photonic://library`, the [standard library](../library/README.md). `//spectrum:test` checks that the primer's grammar is the parser's.

## Verification

`//spectrum:test` checks exploration counts and handles on the conjunction and its edits; that every scope is credited to the rule that opens it; every claim kind with its evidence, including an `inevitable` counterexample that avoids every match; occurrence lineage through remainder, witness, held and produced occurrences; that reordering keeps keys and handles while renaming keeps handles and changes the key; every question's answer through the JSON protocol, checked against its declared output schema; that unknown fields, keys with settings and goals outside path mode are refused; `miss` assignments that a greedy choice gets wrong and exact targets with and without their rules; `compare` counts beyond its limit; and the port of `shape`. `//command:test` runs the commands and their exit codes, and sessions of the protocol server in the legacy and stateless revisions, including discovery, batches and each protocol error.

```sh
bazel test -c opt //spectrum:test //command:test
```

## Not yet built

Spectrum does not yet edit programs (`edit`, `reduce` and `mutate`), lint them beyond parsing and lowering, parse through syntax errors, keep explorations between processes, or run in the browser engine. Its keys are 64-bit hashes; a persistent store would need longer ones.
