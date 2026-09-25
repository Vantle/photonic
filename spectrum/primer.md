# Photonic, for agents

Photonic is a language of rules. A program is data and the rules that change it. The runtime explores every configuration those rules can reach, and Spectrum answers questions about what it found.

## Grammar

This is the whole grammar, the one `frontend/parser.rs` compiles:

```
module = { SOI ~ list ~ EOI }
list = { (term ~ ("," ~ term)* ~ ","?)? }
term = { rule+ ~ (join ~ rule*)? | join ~ rule* }
join = _{ factor ~ ("." ~ factor)* }
factor = _{ concept | group }
group = { "(" ~ list ~ ")" }
rule = { "[" ~ list ~ "]" }
concept = @{ (!("(" | ")" | "[" | "]" | "." | "," | WHITESPACE) ~ ANY)+ }
WHITESPACE = _{ " " | "\t" | "\r" | "\n" | "\u{000B}" | "\u{000C}" }
```

Five rules give it meaning.

1. A comma separates, a dot joins, and a space means nothing. Two particles side by side are an error until a dot or a comma says which you mean.
2. A join multiplies its factors: `A.(B, C)` is `A.B, A.C`, and `()` is the empty coherence.
3. A term is brackets beside at most one particle, in any order. Each bracket consumes what it names and becomes every other part of its term, so `[A] [B]` yields the rules `[A] B` and `[B] A`, and `[A] [B] C` adds `[A] C` and `[B] C`. Listed in a program or a scope, those rules are separate terms, as if written with commas; inside a particle, as in `X.([A] [B])` or the input of `[[A] [B]] C`, they are values of that one particle. Several outputs are a group, as in `[A] (B, C)`. A rule joins a particle only inside parentheses: `X.([A] B)`.
4. A group that lists a rule on its own is a scope. Only a rule's output can open one.
5. A rule listed in a program or a scope is live there. Anywhere else it is a value.

There are no keywords, operators or reserved words: `Not`, `->` and `unless` are ordinary atoms.

## Meaning

- An atom means nothing until rules use it. A particle is an unordered collection of occurrences, and repeats count. A coherence is an independent place where one particle lives. A configuration is every coherence and every live rule at one moment.
- A rule matches openly: its input names occurrences, and whatever else the coherence holds is the remainder. Every output of the rule receives the remainder.
- An event is one rule applied to one match. When several events are possible, the runtime explores every one, so a program has a graph of configurations joined by events.
- Occurrences have identity. One remainder handed to several outputs is one occurrence in each.
- A rule can apply to what a configuration can become. That event is inferred: it happens at the original configuration, and its deduction is the path to where the rule matched. Occurrences reached that way are the witness; the rest of the match is exact.
- A scope's own rules send their output to the enclosing scope. Rules of enclosing scopes also apply inside, and their output stays inside.
- Configurations that differ only in how occurrences are named are one configuration.
- Prism asks whether an exact configuration, live rules included, is reached. It answers reached, unreachable after a closed exploration, or unknown when a budget stopped the search.

## Asking Spectrum

- Start with `check` for diagnostics and claims, or `explore` for an overview: counts, end configurations, and how often each rule fired.
- Answers name things by handle: `r2` a rule, `s11` a configuration, `e12` an event, `s11.c0` a coherence, `s11.o1` an occurrence, `s10.f1` a scope frame. Pass handles to `inspect`, `cause`, `miss` and `step`.
- Answers about an exploration carry its key, such as `x91c7f6619ec81b4b`. Pass it as `exploration` instead of the program, and without `mode`, `budget` or `goal`, to ask more questions of the same recording; a session keeps its most recent explorations. `compare` takes a program or a key on each side, as `left` and `right`.
- `mode: path` follows one direct run, as `photonic_test(path = True)` does; `goal: {"configuration": "False.Extra", "preserve": true}` stops it at a configuration.
- Patterns are Photonic: `B` is a coherence holding B, `B.X` one holding both, `B, C` two different coherences, `([A] B)` a coherence holding that rule value, and `[B, C] D` that rule and the events that apply it.
- Claims answer holds, fails or unknown. Before an exploration closes, `reach` can hold, `avoid`, `always` and `inevitable` can fail, and `inevitable` holds when the start already matches; every other answer, and any answer about `outcome`, needs a closed exploration. Unknown means the search could not settle the claim: a budget stopped it, or a direct path follows one run of many. It is never evidence of absence.
- `cause` explains why something is here: a path, a match and its deduction, or an occurrence's lineage back to the event that produced it or to the start. `miss` explains why not: the nearest configurations and what they lack, or why a rule never fires.
- `compare` shows what an edit changed in behavior. Use it to confirm a refactor keeps the configurations it should.
- Handles are numbered in a canonical order, so reordering terms keeps them, and so does renaming atoms whenever the program's shape is found within the symmetry budget. Direct paths, in `path` mode, follow the program as written.
