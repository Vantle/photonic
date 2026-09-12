use crate::program::{Instruction, Output, Scope, Symbol};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

pub fn instruction(value: &Instruction) -> Instruction {
    normalize(value, &BTreeMap::new())
}

pub fn scope(value: &Scope) -> Scope {
    lexical(value, &BTreeMap::new())
}

fn lexical(value: &Scope, outer: &BTreeMap<String, String>) -> Scope {
    let mut rule = value
        .rule
        .iter()
        .map(|value| Arc::new(normalize(value, outer)))
        .collect::<Vec<_>>();
    rule.sort();
    Scope {
        name: value.name.clone(),
        rule,
    }
}

fn mention(value: &Symbol, result: &mut BTreeSet<String>) {
    match value {
        Symbol::Atom(_) | Symbol::Rule(_, Some(_)) => {}
        Symbol::Variable(name) => {
            result.insert(name.clone());
        }
        Symbol::Structure(_, content) => {
            for value in content {
                mention(value, result);
            }
        }
        Symbol::Rule(rule, None) => {
            for value in rule
                .input
                .iter()
                .flatten()
                .chain(rule.negative.iter().flatten().flatten())
                .chain(rule.output.iter().flat_map(|output| &output.particle))
            {
                mention(value, result);
            }
            for scope in rule.output.iter().filter_map(|output| output.body.as_ref()) {
                for rule in &scope.rule {
                    mention(&Symbol::Rule(rule.clone(), None), result);
                }
            }
        }
    }
}

fn binder(value: &[Vec<Symbol>]) -> BTreeSet<String> {
    let mut result = BTreeSet::new();
    for value in value.iter().flatten() {
        mention(value, &mut result);
    }
    result
}

fn unresolved(value: &Symbol, bound: &BTreeSet<String>, result: &mut BTreeSet<String>) {
    match value {
        Symbol::Atom(_) | Symbol::Rule(_, Some(_)) => {}
        Symbol::Variable(name) => {
            if !bound.contains(name) {
                result.insert(name.clone());
            }
        }
        Symbol::Structure(_, content) => {
            for value in content {
                unresolved(value, bound, result);
            }
        }
        Symbol::Rule(rule, None) => free(rule, bound, result),
    }
}

fn free(value: &Instruction, outer: &BTreeSet<String>, result: &mut BTreeSet<String>) {
    let bound = outer.union(&binder(&value.input)).cloned().collect();
    for output in &value.output {
        for value in &output.particle {
            unresolved(value, &bound, result);
        }
        if let Some(scope) = &output.body {
            for rule in &scope.rule {
                free(rule, &bound, result);
            }
        }
    }
}

fn name(count: usize, used: &mut BTreeSet<String>) -> Vec<String> {
    let mut result = Vec::new();
    let mut ordinal = 0;
    while result.len() < count {
        let candidate = format!("${ordinal}");
        ordinal += 1;
        if used.insert(candidate.clone()) {
            result.push(candidate);
        }
    }
    result
}

fn interchange(value: &Instruction, variable: &[String]) -> BTreeMap<String, usize> {
    let baseline = value.substitute(&BTreeMap::new());
    let mut representative = Vec::<String>::new();
    let mut result = BTreeMap::new();
    for name in variable {
        let group = representative
            .iter()
            .position(|previous| {
                let swap = BTreeMap::from([
                    (name.clone(), Symbol::Variable(previous.clone())),
                    (previous.clone(), Symbol::Variable(name.clone())),
                ]);
                value.substitute(&swap) == baseline
            })
            .unwrap_or_else(|| {
                representative.push(name.clone());
                representative.len() - 1
            });
        result.insert(name.clone(), group);
    }
    result
}

fn permutation(
    value: &mut [String],
    position: usize,
    class: &BTreeMap<String, usize>,
    operation: &mut impl FnMut(&[String]),
) {
    if position == value.len() {
        operation(value);
        return;
    }
    let mut seen = BTreeSet::new();
    for index in position..value.len() {
        if !seen.insert(class[&value[index]]) {
            continue;
        }
        value.swap(position, index);
        permutation(value, position + 1, class, operation);
        value.swap(position, index);
    }
}

fn partition(
    value: &Instruction,
    variable: &[String],
    outer: &BTreeMap<String, String>,
    query: bool,
) -> Vec<Vec<String>> {
    let mut used = Symbol::Rule(Arc::new(value.clone()), None).binding();
    used.extend(outer.values().cloned());
    let marker = name(2, &mut used);
    let mut group = BTreeMap::<Instruction, Vec<String>>::new();
    for distinguished in variable {
        let mut mapping = outer.clone();
        if !query {
            mapping.extend(variable.iter().map(|name| {
                (
                    name.clone(),
                    marker[usize::from(name == distinguished)].clone(),
                )
            }));
        }
        let mut negative = mapping.clone();
        if let Some(input) = &value.negative {
            for name in binder(input) {
                if !mapping.contains_key(&name) {
                    let color = usize::from(query && &name == distinguished);
                    negative.insert(name, marker[color].clone());
                }
            }
        }
        let signature = Instruction {
            name: String::new(),
            input: input(&value.input, &mapping),
            negative: value.negative.as_ref().map(|value| input(value, &negative)),
            output: output(&value.output, &mapping),
        };
        group
            .entry(signature)
            .or_default()
            .push(distinguished.clone());
    }
    group.into_values().collect()
}

fn arrangement(
    group: &mut [Vec<String>],
    prefix: &mut Vec<String>,
    class: &BTreeMap<String, usize>,
    operation: &mut impl FnMut(&[String]),
) {
    let Some((head, tail)) = group.split_first_mut() else {
        operation(prefix);
        return;
    };
    permutation(head, 0, class, &mut |choice| {
        let length = prefix.len();
        prefix.extend_from_slice(choice);
        arrangement(tail, prefix, class, operation);
        prefix.truncate(length);
    });
}

fn normalize(value: &Instruction, outer: &BTreeMap<String, String>) -> Instruction {
    if !value.variable() {
        return value.clone();
    }
    let positive = binder(&value.input);
    let local = positive
        .iter()
        .filter(|name| !outer.contains_key(*name))
        .cloned()
        .collect::<Vec<_>>();
    let query = value
        .negative
        .as_ref()
        .map(|value| binder(value))
        .unwrap_or_default()
        .into_iter()
        .filter(|name| !positive.contains(name) && !outer.contains_key(name))
        .collect::<Vec<_>>();
    let mut used = outer.values().cloned().collect::<BTreeSet<_>>();
    free(value, &outer.keys().cloned().collect(), &mut used);
    let target = name(local.len(), &mut used);
    let existential = name(query.len(), &mut used);
    let mut best = None;
    let symmetry = interchange(value, &local);
    let independence = interchange(value, &query);
    let mut group = partition(value, &local, outer, false);
    arrangement(&mut group, &mut Vec::new(), &symmetry, &mut |local| {
        let mut mapping = outer.clone();
        mapping.extend(local.iter().cloned().zip(target.iter().cloned()));
        let mut group = partition(value, &query, &mapping, true);
        arrangement(&mut group, &mut Vec::new(), &independence, &mut |query| {
            let mut negative = mapping.clone();
            negative.extend(query.iter().cloned().zip(existential.iter().cloned()));
            let candidate = Instruction {
                name: value.name.clone(),
                input: input(&value.input, &mapping),
                negative: value.negative.as_ref().map(|value| input(value, &negative)),
                output: output(&value.output, &mapping),
            };
            if best.as_ref().is_none_or(|best| &candidate < best) {
                best = Some(candidate);
            }
        });
    });
    best.expect("a finite binder set has a canonical assignment")
}

fn pattern(value: &Symbol, mapping: &BTreeMap<String, String>) -> Symbol {
    match value {
        Symbol::Atom(_) | Symbol::Rule(_, Some(_)) => value.clone(),
        Symbol::Variable(name) => Symbol::Variable(mapping.get(name).unwrap_or(name).clone()),
        Symbol::Structure(tag, content) => {
            let mut content = content
                .iter()
                .map(|value| pattern(value, mapping))
                .collect::<Vec<_>>();
            content.sort();
            Symbol::Structure(*tag, content)
        }
        Symbol::Rule(rule, None) => {
            let mut output = rule
                .output
                .iter()
                .map(|output| Output {
                    particle: particle(&output.particle, mapping, pattern),
                    body: output.body.as_ref().map(|scope| {
                        let mut rule = scope
                            .rule
                            .iter()
                            .map(|rule| {
                                let Symbol::Rule(rule, _) =
                                    pattern(&Symbol::Rule(rule.clone(), None), mapping)
                                else {
                                    unreachable!()
                                };
                                rule
                            })
                            .collect::<Vec<_>>();
                        rule.sort();
                        Arc::new(Scope {
                            name: scope.name.clone(),
                            rule,
                        })
                    }),
                })
                .collect::<Vec<_>>();
            output.sort();
            Symbol::Rule(
                Arc::new(Instruction {
                    name: rule.name.clone(),
                    input: input(&rule.input, mapping),
                    negative: rule.negative.as_ref().map(|value| input(value, mapping)),
                    output,
                }),
                None,
            )
        }
    }
}

fn expression(value: &Symbol, mapping: &BTreeMap<String, String>) -> Symbol {
    match value {
        Symbol::Atom(_) | Symbol::Rule(_, Some(_)) => value.clone(),
        Symbol::Variable(name) => Symbol::Variable(mapping.get(name).unwrap_or(name).clone()),
        Symbol::Structure(tag, content) => {
            Symbol::Structure(*tag, particle(content, mapping, expression))
        }
        Symbol::Rule(rule, None) => Symbol::Rule(Arc::new(normalize(rule, mapping)), None),
    }
}

fn particle(
    value: &[Symbol],
    mapping: &BTreeMap<String, String>,
    operation: fn(&Symbol, &BTreeMap<String, String>) -> Symbol,
) -> Vec<Symbol> {
    let mut result = value
        .iter()
        .map(|value| operation(value, mapping))
        .collect::<Vec<_>>();
    result.sort();
    result
}

fn input(value: &[Vec<Symbol>], mapping: &BTreeMap<String, String>) -> Vec<Vec<Symbol>> {
    let mut result = value
        .iter()
        .map(|value| particle(value, mapping, pattern))
        .collect::<Vec<_>>();
    result.sort();
    result
}

fn output(value: &[Output], mapping: &BTreeMap<String, String>) -> Vec<Output> {
    let mut result = value
        .iter()
        .map(|output| Output {
            particle: particle(&output.particle, mapping, expression),
            body: output
                .body
                .as_ref()
                .map(|scope| Arc::new(lexical(scope, mapping))),
        })
        .collect::<Vec<_>>();
    result.sort();
    result
}
