# Photonic, for agents

Photonic is a language of rules. A program is data and the rules that change it. The runtime explores every configuration those rules can reach, and Spectrum answers questions about what it found.

## Grammar

This is the whole grammar, the one `frontend/parser.rs` compiles:

```
module = { SOI ~ list ~ EOI }
list = { term? ~ ("," ~ term?)* }
term = { (atom | group | bracket | ".")+ }
group = { "(" ~ list ~ ")" }
bracket = { "[" ~ list ~ "]" }
atom = @{ (!("(" | ")" | "[" | "]" | "." | "," | WHITESPACE | SPACE_SEPARATOR | CONTROL | BIDI_CONTROL | "\u{200B}" | "\u{2060}") ~ ANY)+ }
WHITESPACE = _{ PATTERN_WHITE_SPACE | "\u{FEFF}" }
```

Five laws give it meaning, with no exceptions. Every balanced text is a program; only an unclosed or unmatched bracket, or a character no atom may hold, is an error.

1. A comma separates members, and a missing member is nothing: `A,` is `A` and `A,,B` is `A, B`.
2. Parts of a term side by side join, and a dot only marks the join: `A.B` is `A B`. Joining distributes over groups: `A(B, C)` is `A B, A C`.
3. A term with brackets is a rule. It consumes what its brackets hold, joined like any parts, and produces the rest of the term, or nothing if nothing is left. Parts are unordered, so `C [A]` is `[A] C`, and `[A] [B] C` is `[A B] C`.
4. A group always holds a place: one that lists neither a particle nor a scope holds the empty particle, so `()` is the empty particle and `A.()` is `A`.
5. Brackets aside, a term that is a single group, in the program, in a scope or in what a rule produces, is the group's members, or a scope if it lists a rule. A rule listed in the program or a scope is live there. Anywhere else, joined or inside brackets, parentheses only group and a rule is a value: `X.([A] B)` carries one, `[[A] B] C` consumes one, and `().([A] B)` is a coherence holding nothing else.

There are no keywords, operators or reserved words: `Not`, `->` and `unless` are ordinary atoms. Whitespace is Unicode's pattern whitespace, which never changes, and U+FEFF, the byte order mark. An atom cannot hold any other space, such as U+00A0, nor a control character, U+200B, U+2060 or a direction control such as U+202E; joiners such as U+200D are part of it. Atoms compare by code point, without normalization.

## Meaning

- An atom means nothing until rules use it. A particle is an unordered collection of occurrences, and repeats count. A coherence is an independent place where one particle lives. A configuration is every coherence and every live rule at one moment.
- A rule matches openly: its input names occurrences, and whatever else the coherence holds is the remainder. Every output of the rule receives the remainder.
- An event is one rule applied to one match. When several events are possible, the runtime explores every one, so a program has a graph of configurations joined by events.
- Occurrences have identity. One remainder handed to several outputs is one occurrence in each.
- A rule can apply to what a configuration can become. That event is inferred: it happens at the original configuration, and its deduction is the path to where the rule matched. Occurrences reached that way are the witness; the rest of the match is exact. Inference reaches configurations that no run of events one at a time reaches; plain mode explores only those runs.
- A rule value in a coherence is live there: it applies to what that coherence holds, and firing reads it without consuming it. A rule value remembers the scope that made it, so equal code made in different scopes gives different values, and a rule's input matches a rule value only when both were made in the same scope.
- A scope's own rules send their output to the enclosing scope. Rules of enclosing scopes also apply inside, and their output stays inside. Every coherence a rule introduces receives the remainder, in its scopes as in its other outputs.
- A rule value belongs to the opening of the innermost scope written around it, or to the program, and keeps it wherever it goes; a rule's input matches only rule values that belong where the rule does.
- Configurations that differ only in how occurrences are named are one configuration. Scopes written alike and opened by the same rule, or both by the program, are interchangeable in the same way.
- A search stops at its budgets: work steps, kept configurations and records, and in each configuration its coherences, the occurrences its coherences and scopes hold as values (live rules do not count) and the scopes it has opened. An answer a budget or limit cut says which one. Syntax nests at most 128 levels, and lowering one program has a budget of its own that grows with the source.
- Prism asks whether an exact configuration, live rules and scopes included, is reached. A target is written as a program and names the configuration that program starts in, so its scopes are ones the program opens at the start. It answers reached, unreachable after a closed exploration, or unknown when a budget stopped the search. Text writes every occurrence independently, so a target never says that coherences share one; a configuration whose coherences share an occurrence is found through lineage, not named as a target.

## Asking Spectrum

- Start with `check` for diagnostics and claims, or `explore` for an overview: counts, end configurations, and how often each rule fired.
- Answers name things by handle: `r2` a rule, `s11` a configuration, `e12` an event, `s11.c0` a coherence, `s11.o1` an occurrence, `s10.f1` a scope frame. Pass handles to `inspect`, `cause`, `miss` and `step`.
- Answers about an exploration carry its key, such as `x91c7f6619ec81b4b`. Pass it as `exploration` instead of the program, and without `mode`, `engine`, `budget` or `goal`, to ask more questions of the same recording; a session keeps its most recent explorations. Each such answer also carries the `mode` and `closed`, which is true once every future, or every plain schedule, was explored within the budget; a direct path never closes. `compare` takes a program or a key on each side, as `left` and `right`.
- `mode: path` follows one direct run, as `photonic_test(path = True)` does; `goal: {"configuration": "False.Extra", "preserve": true}` stops it at a configuration.
- `mode: plain` explores every schedule of plain events, the events a configuration's own matches identify, without inference, on Laser. Ask it whether every order of rule applications reaches a result: `inevitable`, `outcome` and `end` then speak of those schedules, and an inferred shortcut cannot strand a run. With `engine: metal` it explores them through the program's net of parts on the GPU, reaching tens of millions of configurations; metal keeps no events, so only `explore` and `check` take it, its ends have no handles, and it answers `end` and `outcome` claims.
- Every future is explored with Laser unless `engine: interpreter` asks for the interpreter: the same configurations, events, handles and answers once the exploration closes, with each engine's own work counts and deductions. Both use every core. Direct paths always use the interpreter.
- Patterns are Photonic, read as programs and matched by containment: `B` is a coherence holding B, `B.X` one holding both, `B, C` two different coherences, `().([A] B)` a coherence holding that rule value, `(K, [K] L)` a scope holding K and that rule, and `[B, C] D` that rule and the events that apply it. Parts listed anywhere in a pattern, at any depth, match different parts, in any order.
- Claims answer holds, fails or unknown. Before an exploration closes, `reach` can hold, `avoid`, `always`, `inevitable` and `end` can fail, and `inevitable` holds when the start already matches; every other answer, and any answer about `outcome`, needs a closed exploration. `end` claims that every run ends, and at a match: a cycle fails it, and so does an end without a match. `exact` reads any claim's pattern as one whole configuration, and `preserve` adds the program's root rules to it. Unknown means the search could not settle the claim: a budget or limit stopped it, and the answer names which and the flag to raise, or a direct path follows one run of many. It is never evidence of absence.
- `cause` explains why something is here: a path, a match and its deduction, or an occurrence's lineage back to the event that produced it or to the start. `miss` explains why not: the nearest configurations and what they lack, or why a rule does not fire.
- `compare` shows what an edit changed in behavior. Use it to confirm a refactor keeps the configurations it should. A side marked open did not close, and one marked path follows one run, so what it lacks may be unexplored.
- Handles are numbered in a canonical order: configurations by their distance from the start and then their canonical form, events by source, target, rule and binding. So reordering terms keeps them, and so does renaming atoms whenever the program's shape is found within the symmetry budget. Direct paths, in `path` mode, follow the program as written.
