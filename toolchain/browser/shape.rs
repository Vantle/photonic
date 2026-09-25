use crate::failure::{Code, Failure, Item};
use crate::request;
use code::atom::Atom;
use serde::{Deserialize, Serialize};
use symmetry::comparison::{Class, compare};
use symmetry::group::element;
use symmetry::structure::{Part, Structure};
use translation::lift;
use translation::vocabulary::Vocabulary;

const PROGRAM: usize = 4;
const ELEMENT: usize = 64;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Comparison {
    program: Vec<String>,
}

#[derive(Serialize)]
pub struct Partition {
    shape: Vec<Shape>,
}

#[derive(Serialize)]
struct Shape {
    member: Vec<usize>,
    atom: Vec<Row>,
    initial: Vec<String>,
    rule: Vec<String>,
    size: String,
    block: Vec<Vec<String>>,
    symmetry: Vec<Vec<Vec<String>>>,
}

#[derive(Serialize)]
struct Row {
    letter: String,
    name: Vec<String>,
}

fn parse(index: usize, source: &str, vocabulary: &mut Vocabulary) -> Result<Structure, Failure> {
    let program = photonic::lowering::parse(source).map_err(|error| {
        Failure::located(Code::Source, &error, source).within(Item::Program(index))
    })?;
    let (program, configuration) = lift::program(&program, vocabulary)
        .map_err(|error| Failure::from(error).within(Item::Program(index)))?;
    Ok(Structure {
        part: vec![Part {
            role: 0,
            program,
            configuration,
        }],
        pin: Vec::new(),
    })
}

fn shape(class: &Class, vocabulary: &Vocabulary) -> Result<Shape, Failure> {
    let letter = Vocabulary::alphabet(class.atom.len())?;
    let name = |atom: Atom| letter.name(atom).to_owned();
    let form = &class.form.part[0];
    Ok(Shape {
        member: class.member.clone(),
        atom: class
            .atom
            .iter()
            .enumerate()
            .map(|(position, row)| Row {
                letter: name(Atom(position as u16)),
                name: row
                    .iter()
                    .map(|&atom| vocabulary.name(atom).to_owned())
                    .collect(),
            })
            .collect(),
        initial: translation::emit::configuration(&form.configuration, &letter)
            .iter()
            .map(|particle| photonic::text::coherence(particle))
            .collect(),
        rule: form
            .program
            .rule()
            .iter()
            .map(|rule| translation::text::rule(rule, &letter))
            .collect(),
        size: class.size.to_string(),
        block: class
            .block
            .iter()
            .map(|block| block.iter().map(|&atom| name(atom)).collect())
            .collect(),
        symmetry: element(&class.generator, ELEMENT)
            .unwrap_or_else(|| class.generator.clone())
            .iter()
            .map(|permutation| {
                permutation
                    .cycle()
                    .iter()
                    .map(|cycle| cycle.iter().map(|&atom| name(atom)).collect())
                    .collect()
            })
            .collect(),
    })
}

pub fn partition(input: &str) -> Result<Partition, Failure> {
    let comparison: Comparison = request::read(input)?;
    if comparison.program.is_empty() || comparison.program.len() > PROGRAM {
        return Err(Failure::new(Code::Request, "Compare one to four programs."));
    }
    let mut vocabulary = Vocabulary::default();
    let structure = comparison
        .program
        .iter()
        .enumerate()
        .map(|(index, source)| parse(index, source, &mut vocabulary))
        .collect::<Result<Vec<_>, _>>()?;
    let class = compare(&structure, crate::limit::SYMMETRY).map_err(|exhausted| {
        Failure::new(
            Code::Budget,
            format!(
                "The symmetry search stopped at depth {} after {} nodes.",
                exhausted.depth, exhausted.node
            ),
        )
    })?;
    Ok(Partition {
        shape: class
            .iter()
            .map(|entry| shape(entry, &vocabulary))
            .collect::<Result<_, _>>()?,
    })
}
