use crate::argument::Analysis;
use code::atom::Atom;
use miette::IntoDiagnostic;
use photonic::source;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use symmetry::group::{Permutation, element, orbit};
use symmetry::rename;
use symmetry::search::Exhausted;
use symmetry::structure::{Part, Structure, Symmetry};
use translation::lift::{self, Naming};
use translation::text;
use translation::vocabulary::Vocabulary;

const ELEMENT: usize = 64;

#[derive(Serialize)]
struct Report {
    atom: usize,
    rule: usize,
    size: String,
    node: usize,
    block: Vec<Vec<String>>,
    symmetry: Vec<Vec<Vec<String>>>,
    orbit: Vec<Vec<String>>,
}

#[derive(Serialize)]
struct Class {
    member: Vec<String>,
    atom: Vec<Vec<String>>,
    size: String,
}

#[derive(Serialize)]
struct Form {
    shape: String,
    atom: Vec<(String, String)>,
    program: String,
    target: Option<String>,
}

fn part(role: u32, source: &source::Program, vocabulary: &mut Vocabulary) -> miette::Result<Part> {
    let (program, configuration) = lift::program(source, vocabulary)?;
    Ok(Part {
        role,
        program,
        configuration,
    })
}

fn load(
    path: &Path,
    analysis: &Analysis,
    vocabulary: &mut Vocabulary,
) -> miette::Result<Structure> {
    let mut source = source::Program::default();
    for library in &analysis.library {
        source.declare(photonic::lowering::read(library)?, library.display())?;
    }
    source.append(crate::program(path, analysis.format)?);
    let mut part = vec![self::part(0, &source, vocabulary)?];
    if let Some(target) = &analysis.target {
        let target = crate::program(target, analysis.format)?;
        part.push(self::part(1, &target, vocabulary)?);
    }
    let pin = analysis
        .fix
        .iter()
        .map(|name| vocabulary.atom(name))
        .collect::<Result<_, _>>()?;
    Ok(Structure { part, pin })
}

fn exhausted(exhausted: Exhausted) -> miette::Report {
    miette::miette!(
        "the symmetry search stopped at depth {} after {} nodes; raise --budget",
        exhausted.depth,
        exhausted.node
    )
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

pub fn symmetry(path: PathBuf, analysis: Analysis) -> miette::Result<()> {
    let mut vocabulary = Vocabulary::default();
    let structure = load(&path, &analysis, &mut vocabulary)?;
    let symmetry = structure.symmetry(analysis.budget).map_err(exhausted)?;
    let rule = structure.part[0].program.rule();
    let name = display(&symmetry, &vocabulary);
    let report = Report {
        atom: symmetry.atom.len(),
        rule: rule.len(),
        size: symmetry.size.to_string(),
        node: symmetry.node,
        block: symmetry
            .block
            .iter()
            .map(|block| named(block, &vocabulary))
            .collect(),
        symmetry: element(&symmetry.generator, ELEMENT)
            .unwrap_or_else(|| symmetry.generator.clone())
            .iter()
            .map(|permutation| cycle(permutation, &name))
            .collect(),
        orbit: orbit(rule, &symmetry.generator, |rule, permutation| {
            rename::rule(rule, &|atom| permutation.image(atom))
        })
        .into_iter()
        .filter(|member| member.len() > 1)
        .map(|member| {
            member
                .iter()
                .map(|&index| text::rule(&rule[index], &vocabulary))
                .collect()
        })
        .collect(),
    };
    if analysis.json {
        return crate::output::write(&report, false);
    }
    let mut output = std::io::stdout().lock();
    writeln!(
        output,
        "{} atoms, {} rules, {} automorphisms",
        report.atom, report.rule, report.size
    )
    .into_diagnostic()?;
    for block in &report.block {
        writeln!(output, "Block {}", block.join(".")).into_diagnostic()?;
    }
    for permutation in &report.symmetry {
        let cycle = permutation
            .iter()
            .map(|cycle| format!("({})", cycle.join(" ")))
            .collect::<String>();
        writeln!(output, "Symmetry {cycle}").into_diagnostic()?;
    }
    for member in &report.orbit {
        writeln!(output, "Orbit").into_diagnostic()?;
        for rule in member {
            writeln!(output, "    {rule}").into_diagnostic()?;
        }
    }
    Ok(())
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

pub fn compare(path: Vec<PathBuf>, analysis: Analysis) -> miette::Result<()> {
    let mut vocabulary = Vocabulary::default();
    let structure = path
        .iter()
        .map(|path| load(path, &analysis, &mut vocabulary))
        .collect::<miette::Result<Vec<_>>>()?;
    let class = symmetry::comparison::compare(&structure, analysis.budget).map_err(exhausted)?;
    if analysis.json {
        let report = class
            .iter()
            .map(|class| Class {
                member: class
                    .member
                    .iter()
                    .map(|&index| path[index].display().to_string())
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
                size: class.size.to_string(),
            })
            .collect::<Vec<_>>();
        crate::output::write(&report, false)?;
    } else {
        let mut output = std::io::stdout().lock();
        for class in &class {
            writeln!(output, "{}", path[class.member[0]].display()).into_diagnostic()?;
            for (position, &index) in class.member.iter().enumerate().skip(1) {
                let pair = class
                    .atom
                    .iter()
                    .map(|row| (row[0], row[position]))
                    .collect::<Vec<_>>();
                writeln!(
                    output,
                    "  = {} by {}",
                    path[index].display(),
                    describe(&pair, &vocabulary)
                )
                .into_diagnostic()?;
            }
        }
    }
    if class.len() > 1 {
        return Err(miette::miette!(
            "the programs fall into {} shapes",
            class.len()
        ));
    }
    Ok(())
}

fn source(part: &Part, vocabulary: &Vocabulary) -> String {
    let coherence = text::configuration(&part.configuration, vocabulary);
    let initial = (!coherence.is_empty()).then(|| coherence + ",\n");
    initial.unwrap_or_default() + &text::program(&part.program, vocabulary)
}

pub fn form(path: PathBuf, analysis: Analysis) -> miette::Result<()> {
    let mut vocabulary = Vocabulary::default();
    let structure = load(&path, &analysis, &mut vocabulary)?;
    let symmetry = structure.symmetry(analysis.budget).map_err(exhausted)?;
    let canonical = symmetry.form(&structure);
    let letter = Vocabulary::alphabet(symmetry.atom.len());
    let report = Form {
        shape: format!("{:016x}", symmetry.fingerprint()),
        atom: symmetry
            .atom
            .iter()
            .enumerate()
            .map(|(position, &atom)| {
                (
                    vocabulary.name(atom).to_owned(),
                    letter.name(Atom(position as u16)).to_owned(),
                )
            })
            .collect(),
        program: source(&canonical.part[0], &letter),
        target: canonical.part.get(1).map(|part| source(part, &letter)),
    };
    if analysis.json {
        return crate::output::write(&report, false);
    }
    let mut output = std::io::stdout().lock();
    writeln!(output, "Shape {}", report.shape).into_diagnostic()?;
    write!(output, "{}", report.program).into_diagnostic()?;
    if let Some(target) = &report.target {
        writeln!(output, "Target").into_diagnostic()?;
        write!(output, "{target}").into_diagnostic()?;
    }
    Ok(())
}
