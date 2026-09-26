use crate::context::Context;
use crate::failure::{Code, Failure};
use crate::render;
use crate::subject::Subject;
use code::atom::Atom;
use frontend::source;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use symmetry::analysis::analyze;
use symmetry::group::{Permutation, element};
use symmetry::pattern::Pattern;
use symmetry::search::Exhausted;
use symmetry::statement::Statement;
use symmetry::structure::{Part, Structure, Symmetry};
use translation::lift;
use translation::text;
use translation::vocabulary::Vocabulary;

const ELEMENT: usize = 64;

pub const NODE: usize = 1_000_000;

fn node() -> usize {
    NODE
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[schemars(
    description = "Describe programs up to the names of their atoms. One program: its shape key, canonical form, symmetries, orbits and rules that recur under other names. Several: the groups that share a shape, with the renaming between members."
)]
pub struct Request {
    #[schemars(description = "One program to describe, or several to group by shape.")]
    pub program: Vec<Subject>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "A target configuration that every renaming must also preserve.")]
    pub target: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(description = "Atoms whose names every renaming must keep.")]
    pub fix: Vec<String>,
    #[serde(default = "node")]
    #[schemars(description = "Search tree nodes the symmetry engine may visit.")]
    pub node: usize,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub(crate) struct Letter {
    pub(crate) name: String,
    pub(crate) letter: String,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub(crate) struct Copy {
    pub(crate) atom: Vec<String>,
    pub(crate) statement: Vec<String>,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub(crate) struct Class {
    pub(crate) member: Vec<usize>,
    pub(crate) name: Vec<String>,
    pub(crate) atom: Vec<Vec<String>>,
    pub(crate) renaming: Vec<String>,
    pub(crate) size: String,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub(crate) struct Form {
    pub(crate) shape: String,
    pub(crate) atom: Vec<Letter>,
    pub(crate) text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) target: Option<String>,
    pub(crate) rule: usize,
    pub(crate) size: String,
    pub(crate) node: usize,
    pub(crate) block: Vec<Vec<String>>,
    pub(crate) symmetry: Vec<Vec<Vec<String>>>,
    pub(crate) orbit: Vec<Vec<String>>,
    pub(crate) pattern: Vec<Vec<Copy>>,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub struct Answer {
    pub(crate) class: Vec<Class>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) form: Option<Form>,
}

fn exhausted(exhausted: Exhausted) -> Failure {
    let message = match exhausted {
        Exhausted::Node(node) => {
            format!("the symmetry search stopped after {node} nodes; raise node")
        }
        Exhausted::Depth(depth) => format!(
            "the symmetry search stopped at depth {depth}, its limit; the program has too many interchangeable parts to canonicalize"
        ),
    };
    Failure::new(Code::Shape, message)
}

fn part(source: &source::Program, vocabulary: &mut Vocabulary) -> Result<Part, Failure> {
    let (program, configuration) = lift::program(source, vocabulary)
        .map_err(|error| Failure::new(Code::Shape, error.to_string()))?;
    Ok(Part {
        program,
        configuration,
    })
}

fn structure(
    source: &source::Program,
    target: Option<&source::Program>,
    vocabulary: &mut Vocabulary,
) -> Result<Structure, Failure> {
    Ok(Structure {
        program: part(source, vocabulary)?,
        target: target.map(|target| part(target, vocabulary)).transpose()?,
        pin: Vec::new(),
    })
}

fn named(atom: &[Atom], vocabulary: &Vocabulary) -> Vec<String> {
    let mut name = atom
        .iter()
        .map(|&atom| vocabulary.name(atom).to_owned())
        .collect::<Vec<_>>();
    name.sort();
    name
}

fn display(symmetry: &Symmetry, vocabulary: &Vocabulary) -> BTreeMap<Atom, String> {
    let mut result = symmetry
        .atom
        .iter()
        .map(|&atom| (atom, vocabulary.name(atom).to_owned()))
        .collect::<BTreeMap<_, _>>();
    for block in &symmetry.block {
        result.insert(block[0], named(block, vocabulary).join("."));
        for member in &block[1..] {
            result.remove(member);
        }
    }
    result
}

fn cycle(permutation: &Permutation, name: &BTreeMap<Atom, String>) -> Vec<Vec<String>> {
    permutation
        .cycle()
        .into_iter()
        .filter(|cycle| name.contains_key(&cycle[0]))
        .map(|cycle| cycle.iter().map(|atom| name[atom].clone()).collect())
        .collect()
}

fn statement(structure: &Structure) -> Vec<Statement> {
    let part = &structure.program;
    part.program
        .rule()
        .iter()
        .cloned()
        .map(Statement::Rule)
        .chain(
            part.configuration
                .coherence()
                .iter()
                .cloned()
                .map(Statement::Coherence),
        )
        .chain(part.program.scope().iter().cloned().map(Statement::Scope))
        .collect()
}

fn write(statement: &Statement, vocabulary: &Vocabulary) -> String {
    match statement {
        Statement::Rule(rule) => text::rule(rule, vocabulary),
        Statement::Coherence(particle) => frontend::text::coherence(
            &translation::emit::configuration(
                &code::configuration::Configuration::from(vec![particle.clone()]),
                vocabulary,
            )[0],
        ),
        Statement::Scope(scope) => text::scope(scope, vocabulary),
    }
}

fn pattern(pattern: &Pattern, statement: &[Statement], vocabulary: &Vocabulary) -> Vec<Copy> {
    let varying = pattern.varying();
    pattern
        .occurrence
        .iter()
        .map(|occurrence| Copy {
            atom: varying
                .iter()
                .map(|&position| vocabulary.name(occurrence.atom[position]).to_owned())
                .collect(),
            statement: occurrence
                .statement
                .iter()
                .map(|&index| write(&statement[index], vocabulary))
                .collect(),
        })
        .collect()
}

fn describe(pair: &[(Atom, Atom)], vocabulary: &Vocabulary) -> String {
    let moved = pair
        .iter()
        .copied()
        .filter(|(from, to)| from != to)
        .collect::<BTreeMap<_, _>>();
    let image = moved.values().copied().collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    let mut piece = Vec::new();
    for &start in moved.keys().filter(|atom| !image.contains(atom)) {
        let mut chain = vec![vocabulary.name(start)];
        let mut next = start;
        while let Some(&after) = moved.get(&next) {
            seen.insert(next);
            chain.push(vocabulary.name(after));
            next = after;
        }
        piece.push(chain.join(" → "));
    }
    for &start in moved.keys() {
        if !seen.insert(start) {
            continue;
        }
        let mut cycle = vec![vocabulary.name(start)];
        let mut next = moved[&start];
        while seen.insert(next) {
            cycle.push(vocabulary.name(next));
            next = moved[&next];
        }
        piece.push(format!("({})", cycle.join(" ")));
    }
    if piece.is_empty() {
        return "identical".to_owned();
    }
    piece.join(" ")
}

fn source(part: &Part, vocabulary: &Vocabulary) -> String {
    let coherence = text::configuration(&part.configuration, vocabulary);
    let initial = (!coherence.is_empty()).then(|| coherence + ",\n");
    initial.unwrap_or_default() + &text::program(&part.program, vocabulary)
}

fn form(structure: &Structure, node: usize, vocabulary: &Vocabulary) -> Result<Form, Failure> {
    let statement = self::statement(structure);
    let result = analyze(structure, &statement, node).map_err(exhausted)?;
    let symmetry = &result.symmetry;
    let canonical = &symmetry.form;
    let letter = Vocabulary::alphabet(symmetry.atom.len())
        .map_err(|error| Failure::new(Code::Shape, error.to_string()))?;
    let name = display(symmetry, vocabulary);
    Ok(Form {
        shape: format!("{:016x}", symmetry.fingerprint()),
        atom: symmetry
            .atom
            .iter()
            .enumerate()
            .map(|(position, &atom)| Letter {
                name: vocabulary.name(atom).to_owned(),
                letter: letter.name(Atom(position as u16)).to_owned(),
            })
            .collect(),
        text: source(&canonical.program, &letter),
        target: canonical.target.as_ref().map(|part| source(part, &letter)),
        rule: structure.program.program.rule().len(),
        size: symmetry.size.to_string(),
        node: symmetry.node,
        block: symmetry
            .block
            .iter()
            .map(|block| named(block, vocabulary))
            .collect(),
        symmetry: element(&symmetry.generator, ELEMENT)
            .unwrap_or_else(|| symmetry.generator.clone())
            .iter()
            .map(|permutation| cycle(permutation, &name))
            .collect(),
        orbit: result
            .statement
            .iter()
            .map(|member| {
                member
                    .iter()
                    .map(|&index| write(&statement[index], vocabulary))
                    .collect()
            })
            .collect(),
        pattern: result
            .pattern
            .iter()
            .map(|entry| pattern(entry, &statement, vocabulary))
            .collect(),
    })
}

pub(crate) fn answer(request: &Request, context: &Context<'_>) -> Result<Answer, Failure> {
    if request.program.is_empty() {
        return Err(Failure::new(Code::Request, "give at least one program"));
    }
    let target = request
        .target
        .as_deref()
        .map(|text| crate::subject::lower("target", text, Code::Target))
        .transpose()?;
    let mut vocabulary = Vocabulary::default();
    let structure = request
        .program
        .iter()
        .map(|subject| {
            let source = subject.assemble(context.reader)?;
            self::structure(&source, target.as_ref(), &mut vocabulary)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let fix = request
        .fix
        .iter()
        .map(|name| {
            vocabulary.find(name).ok_or_else(|| {
                Failure::new(Code::Shape, format!("no program names the atom {name}"))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let structure = structure
        .into_iter()
        .map(|structure| Structure {
            pin: fix.clone(),
            ..structure
        })
        .collect::<Vec<_>>();
    let class = symmetry::comparison::compare(&structure, request.node).map_err(exhausted)?;
    let title = request
        .program
        .iter()
        .map(|subject| {
            subject
                .file
                .first()
                .cloned()
                .unwrap_or_else(|| "source".to_owned())
        })
        .collect::<Vec<_>>();
    let class = class
        .iter()
        .map(|class| Class {
            member: class.member.clone(),
            name: class
                .member
                .iter()
                .map(|&index| title[index].clone())
                .collect(),
            atom: class
                .atom
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|&atom| vocabulary.name(atom).to_owned())
                        .collect()
                })
                .collect(),
            renaming: (1..class.member.len())
                .map(|position| {
                    let pair = class
                        .atom
                        .iter()
                        .map(|row| (row[0], row[position]))
                        .collect::<Vec<_>>();
                    describe(&pair, &vocabulary)
                })
                .collect(),
            size: class.size.to_string(),
        })
        .collect();
    let form = match structure.as_slice() {
        [single] => Some(form(single, request.node, &vocabulary)?),
        _ => None,
    };
    Ok(Answer { class, form })
}

impl Answer {
    pub(crate) fn passed(&self) -> bool {
        self.class.len() <= 1
    }

    pub(crate) fn text(&self) -> String {
        let mut line = Vec::new();
        if let Some(form) = &self.form {
            let automorphism = if form.size == "1" {
                "automorphism"
            } else {
                "automorphisms"
            };
            line.push(format!(
                "shape {} · {}, {}, {} {automorphism}",
                form.shape,
                render::count(form.atom.len(), "atom"),
                render::count(form.rule, "rule"),
                form.size
            ));
            line.push(
                form.atom
                    .iter()
                    .map(|entry| format!("{} {}", entry.letter, entry.name))
                    .collect::<Vec<_>>()
                    .join(" · "),
            );
            for block in &form.block {
                line.push(format!("block {}", block.join(".")));
            }
            for permutation in &form.symmetry {
                line.push(format!(
                    "symmetry {}",
                    permutation
                        .iter()
                        .map(|cycle| format!("({})", cycle.join(" ")))
                        .collect::<String>()
                ));
            }
            for member in &form.orbit {
                line.push("orbit".to_owned());
                for statement in member {
                    line.push(format!("    {statement}"));
                }
            }
            for occurrence in &form.pattern {
                line.push(format!(
                    "pattern of {} in {} copies",
                    render::count(
                        occurrence.first().map_or(0, |copy| copy.statement.len()),
                        "statement"
                    ),
                    occurrence.len()
                ));
                for copy in occurrence {
                    let label = if copy.atom.is_empty() {
                        "identical".to_owned()
                    } else {
                        copy.atom.join(" ")
                    };
                    line.push(format!("  {label}"));
                    for statement in &copy.statement {
                        line.push(format!("    {statement}"));
                    }
                }
            }
            line.push(form.text.trim_end().to_owned());
            if let Some(target) = &form.target {
                line.push("target".to_owned());
                line.push(target.trim_end().to_owned());
            }
            return line.join("\n");
        }
        line.push(format!(
            "{} in {}",
            render::count(
                self.class
                    .iter()
                    .map(|class| class.member.len())
                    .sum::<usize>(),
                "program"
            ),
            render::count(self.class.len(), "shape")
        ));
        for class in &self.class {
            line.push(class.name[0].clone());
            for (position, renaming) in class.renaming.iter().enumerate() {
                line.push(format!("  = {} by {renaming}", class.name[position + 1]));
            }
        }
        line.join("\n")
    }
}
