use code::analogy::{correspond, partition};
use code::atom::Atom;
use code::output::Output;
use code::particle::Particle;
use code::program::Program;
use code::rule::Rule;
use code::tree::{Place, transform, walk};
use code::value::Value;
use random::Generator;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Side {
    Input,
    Output,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Action {
    Stop,
    Create {
        atom: Atom,
    },
    Duplicate {
        rule: usize,
    },
    Delete {
        rule: usize,
    },
    Open {
        rule: usize,
        side: Side,
    },
    Close {
        rule: usize,
        side: Side,
        particle: usize,
    },
    Insert {
        rule: usize,
        side: Side,
        particle: usize,
        atom: Atom,
    },
    Remove {
        rule: usize,
        side: Side,
        particle: usize,
        atom: Atom,
    },
    Nest {
        rule: usize,
        side: Side,
        particle: usize,
        atom: Atom,
    },
    Enclose {
        rule: usize,
        output: usize,
        atom: Atom,
    },
    Detach {
        rule: usize,
        atom: Atom,
    },
    Analogy(Local),
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Local {
    Delete {
        rule: usize,
    },
    Detach {
        rule: usize,
        atom: Atom,
    },
    Close {
        rule: usize,
        side: Side,
        particle: usize,
    },
    Remove {
        rule: usize,
        side: Side,
        particle: usize,
        atom: Atom,
    },
}

impl From<Local> for Action {
    fn from(local: Local) -> Self {
        match local {
            Local::Delete { rule } => Self::Delete { rule },
            Local::Detach { rule, atom } => Self::Detach { rule, atom },
            Local::Close {
                rule,
                side,
                particle,
            } => Self::Close {
                rule,
                side,
                particle,
            },
            Local::Remove {
                rule,
                side,
                particle,
                atom,
            } => Self::Remove {
                rule,
                side,
                particle,
                atom,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Bound {
    pub rule: usize,
    pub particle: usize,
    pub atom: usize,
    pub depth: usize,
}

impl Default for Bound {
    fn default() -> Self {
        Self {
            rule: 20,
            particle: 4,
            atom: 6,
            depth: 0,
        }
    }
}

pub fn seed(atom: Atom) -> Rule {
    Rule::new(
        vec![Particle::atom(&[atom])],
        vec![Output::plain(Particle::default())],
    )
}

fn input(rule: &Rule, change: impl FnOnce(&mut Vec<Particle>)) -> Rule {
    let mut input = rule.input().to_vec();
    change(&mut input);
    Rule::new(input, rule.output().to_vec())
}

fn output(rule: &Rule, change: impl FnOnce(&mut Vec<Output>)) -> Rule {
    let mut output = rule.output().to_vec();
    change(&mut output);
    Rule::new(rule.input().to_vec(), output)
}

fn modify(program: &Program, index: usize, change: impl FnOnce(&Rule) -> Rule) -> Program {
    transform(program, index, |rule| vec![change(rule)])
}

pub fn apply(program: &Program, action: Action) -> Program {
    match action {
        Action::Stop => program.clone(),
        Action::Create { atom } => {
            let mut rule = program.rule().to_vec();
            rule.push(seed(atom));
            Program::from(rule)
        }
        Action::Duplicate { rule } => {
            transform(program, rule, |value| vec![value.clone(), value.clone()])
        }
        Action::Delete { rule } => transform(program, rule, |_| Vec::new()),
        Action::Open { rule, side } => match side {
            Side::Input => modify(program, rule, |value| {
                input(value, |particle| particle.push(Particle::default()))
            }),
            Side::Output => modify(program, rule, |value| {
                output(value, |output| {
                    output.push(Output::plain(Particle::default()))
                })
            }),
        },
        Action::Close {
            rule,
            side,
            particle,
        } => modify(program, rule, |value| close(value, side, particle)),
        Action::Insert {
            rule,
            side,
            particle,
            atom,
        } => place(program, rule, side, particle, Value::Atom(atom)),
        Action::Nest {
            rule,
            side,
            particle,
            atom,
        } => place(
            program,
            rule,
            side,
            particle,
            Value::Rule(Box::new(seed(atom))),
        ),
        Action::Remove {
            rule,
            side,
            particle,
            atom,
        } => modify(program, rule, |value| remove(value, side, particle, atom)),
        Action::Enclose {
            rule,
            output: index,
            atom,
        } => modify(program, rule, |value| {
            output(value, |entry| {
                let mut body = entry[index]
                    .body()
                    .map(<[Rule]>::to_vec)
                    .unwrap_or_default();
                body.push(seed(atom));
                entry[index] = Output::new(entry[index].particle().clone(), Some(body));
            })
        }),
        Action::Detach { rule, atom } => modify(program, rule, |value| detach(value, atom)),
        Action::Analogy(local) => analogy(program, local),
    }
}

fn close(rule: &Rule, side: Side, particle: usize) -> Rule {
    match side {
        Side::Input => input(rule, |entry| {
            entry.remove(particle);
        }),
        Side::Output => output(rule, |output| {
            output.remove(particle);
        }),
    }
}

fn remove(rule: &Rule, side: Side, particle: usize, atom: Atom) -> Rule {
    let target = Value::Atom(atom);
    match side {
        Side::Input => input(rule, |entry| {
            if let Some(reduced) = entry[particle].remove(&target) {
                entry[particle] = reduced;
            }
        }),
        Side::Output => output(rule, |output| {
            if let Some(reduced) = output[particle].particle().remove(&target) {
                output[particle] =
                    Output::new(reduced, output[particle].body().map(<[Rule]>::to_vec));
            }
        }),
    }
}

fn detach(rule: &Rule, atom: Atom) -> Rule {
    let target = Value::Atom(atom);
    Rule::new(
        rule.input()
            .iter()
            .filter(|particle| !particle.value().contains(&target))
            .cloned()
            .collect(),
        rule.output()
            .iter()
            .filter(|output| !output.particle().value().contains(&target))
            .cloned()
            .collect(),
    )
}

fn top(program: &Program) -> Vec<usize> {
    walk(program)
        .iter()
        .enumerate()
        .filter(|(_, node)| node.place == Place::Program)
        .map(|(index, _)| index)
        .collect()
}

impl Local {
    fn rule(self) -> usize {
        match self {
            Self::Delete { rule }
            | Self::Detach { rule, .. }
            | Self::Close { rule, .. }
            | Self::Remove { rule, .. } => rule,
        }
    }
}

fn analogy(program: &Program, local: Local) -> Program {
    let index = top(program);
    let Some(position) = index.iter().position(|&entry| entry == local.rule()) else {
        return program.clone();
    };
    let source = &program.rule()[position];
    let rule: Vec<Rule> = program
        .rule()
        .iter()
        .enumerate()
        .filter_map(|(place, target)| {
            let Some(map) = correspond(source, target) else {
                return Some(target.clone());
            };
            let particle = |side: Side, particle: usize| {
                if place == position {
                    return particle;
                }
                match side {
                    Side::Input => map.input[particle],
                    Side::Output => map.output[particle],
                }
            };
            let atom = |atom: Atom| {
                if place == position {
                    atom
                } else {
                    map.atom[&atom]
                }
            };
            match local {
                Local::Delete { .. } => None,
                Local::Detach { atom: value, .. } => Some(detach(target, atom(value))),
                Local::Close {
                    side,
                    particle: value,
                    ..
                } => Some(close(target, side, particle(side, value))),
                Local::Remove {
                    side,
                    particle: value,
                    atom: symbol,
                    ..
                } => Some(remove(target, side, particle(side, value), atom(symbol))),
            }
        })
        .collect();
    Program::from(rule)
}

fn place(program: &Program, rule: usize, side: Side, particle: usize, value: Value) -> Program {
    match side {
        Side::Input => modify(program, rule, |current| {
            input(current, |entry| {
                if particle == entry.len() {
                    entry.push(Particle::from(vec![value]));
                } else {
                    entry[particle] = entry[particle].insert(value);
                }
            })
        }),
        Side::Output => modify(program, rule, |current| {
            output(current, |output| {
                if particle == output.len() {
                    output.push(Output::plain(Particle::from(vec![value])));
                } else {
                    output[particle] = Output::new(
                        output[particle].particle().insert(value),
                        output[particle].body().map(<[Rule]>::to_vec),
                    );
                }
            })
        }),
    }
}

fn particle(rule: &Rule, side: Side) -> Vec<&Particle> {
    match side {
        Side::Input => rule.input().iter().collect(),
        Side::Output => rule.output().iter().map(Output::particle).collect(),
    }
}

fn local(index: usize, rule: &Rule) -> Vec<Local> {
    let mut result = vec![Local::Delete { rule: index }];
    for side in [Side::Input, Side::Output] {
        let value = particle(rule, side);
        for (position, particle) in value.iter().enumerate() {
            if side == Side::Output || value.len() > 1 {
                result.push(Local::Close {
                    rule: index,
                    side,
                    particle: position,
                });
            }
            let mut present = particle
                .value()
                .iter()
                .filter_map(Value::atom)
                .collect::<Vec<_>>();
            present.dedup();
            result.extend(present.into_iter().map(|atom| Local::Remove {
                rule: index,
                side,
                particle: position,
                atom,
            }));
        }
    }
    let mut mentioned = rule
        .input()
        .iter()
        .chain(rule.output().iter().map(Output::particle))
        .flat_map(|particle| particle.value().iter().filter_map(Value::atom))
        .collect::<Vec<_>>();
    mentioned.sort_unstable();
    mentioned.dedup();
    result.extend(
        mentioned
            .into_iter()
            .filter(|&atom| {
                rule.input()
                    .iter()
                    .any(|particle| !particle.value().contains(&Value::Atom(atom)))
            })
            .map(|atom| Local::Detach { rule: index, atom }),
    );
    result
}

pub fn legal(program: &Program, vocabulary: usize, bound: &Bound) -> Vec<Action> {
    let atom = || (0..vocabulary).map(|index| Atom(index as u16));
    let node = walk(program);
    let growth = node.len() < bound.rule;
    let mut result = vec![Action::Stop];
    if growth {
        result.extend(atom().map(|atom| Action::Create { atom }));
    }
    for (index, entry) in node.iter().enumerate() {
        let nesting = growth && entry.depth < bound.depth;
        if growth {
            result.push(Action::Duplicate { rule: index });
        }
        result.extend(local(index, entry.rule).into_iter().map(Action::from));
        for side in [Side::Input, Side::Output] {
            let value = particle(entry.rule, side);
            if value.len() < bound.particle {
                result.push(Action::Open { rule: index, side });
                result.extend(atom().map(|atom| Action::Insert {
                    rule: index,
                    side,
                    particle: value.len(),
                    atom,
                }));
                if nesting {
                    result.extend(atom().map(|atom| Action::Nest {
                        rule: index,
                        side,
                        particle: value.len(),
                        atom,
                    }));
                }
            }
            for (position, particle) in value.iter().enumerate() {
                if particle.len() < bound.atom {
                    result.extend(atom().map(|atom| Action::Insert {
                        rule: index,
                        side,
                        particle: position,
                        atom,
                    }));
                    if nesting {
                        result.extend(atom().map(|atom| Action::Nest {
                            rule: index,
                            side,
                            particle: position,
                            atom,
                        }));
                    }
                }
            }
        }
        if nesting {
            for output in 0..entry.rule.output().len() {
                result.extend(atom().map(|atom| Action::Enclose {
                    rule: index,
                    output,
                    atom,
                }));
            }
        }
    }
    let index = top(program);
    let rule = index
        .iter()
        .map(|&entry| node[entry].rule)
        .collect::<Vec<_>>();
    for class in partition(&rule) {
        if class.len() > 1 {
            let representative = index[class[0]];
            result.extend(
                local(representative, node[representative].rule)
                    .into_iter()
                    .map(Action::Analogy),
            );
        }
    }
    result
}

pub fn perturb(
    program: &Program,
    vocabulary: usize,
    bound: &Bound,
    count: usize,
    generator: &mut Generator,
) -> Program {
    let mut current = program.clone();
    for _ in 0..count {
        let option = legal(&current, vocabulary, bound);
        if option.len() < 2 {
            break;
        }
        let choice = option[1 + generator.below(option.len() - 1)];
        current = apply(&current, choice);
    }
    current
}
