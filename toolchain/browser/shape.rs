use crate::failure::{Code, Failure};
use crate::request::bound;
use code::atom::Atom;
use serde::{Deserialize, Serialize};
use symmetry::comparison::{Class, compare};
use symmetry::group::element;
use symmetry::structure::{Part, Structure};
use translation::lift;
use translation::vocabulary::Vocabulary;

const PROGRAM: usize = 4;
const BUDGET: usize = 100_000;
const ELEMENT: usize = 64;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    version: u32,
    program: Vec<String>,
}

#[derive(Serialize)]
pub struct Comparison {
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

fn structure(
    index: usize,
    source: &str,
    vocabulary: &mut Vocabulary,
) -> Result<Structure, Failure> {
    let program = photonic::lowering::parse(source)
        .map_err(|error| Failure::located(Code::Source, &error, source).within(index))?;
    let (program, configuration) = lift::program(&program, vocabulary)
        .map_err(|error| Failure::new(Code::Source, error).within(index))?;
    Ok(Structure {
        part: vec![Part {
            role: 0,
            program,
            configuration,
        }],
        pin: Vec::new(),
    })
}

fn shape(class: &Class, vocabulary: &Vocabulary) -> Shape {
    let letter = Vocabulary::alphabet(class.atom.len());
    let name = |atom: Atom| letter.name(atom).to_owned();
    let form = &class.form.part[0];
    Shape {
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
    }
}

pub fn comparison(input: &str) -> Result<Comparison, Failure> {
    let request: Request =
        serde_json::from_str(bound(input)?).map_err(|error| Failure::new(Code::Request, error))?;
    if request.version != 1 {
        return Err(Failure::new(Code::Version, "unsupported request version"));
    }
    if request.program.is_empty() || request.program.len() > PROGRAM {
        return Err(Failure::new(Code::Request, "compare one to four programs"));
    }
    let mut vocabulary = Vocabulary::default();
    let structure = request
        .program
        .iter()
        .enumerate()
        .map(|(index, source)| self::structure(index, source, &mut vocabulary))
        .collect::<Result<Vec<_>, _>>()?;
    let class = compare(&structure, BUDGET).map_err(|exhausted| {
        Failure::new(
            Code::Size,
            format!(
                "the symmetry search stopped at depth {} after {} nodes",
                exhausted.depth, exhausted.node
            ),
        )
    })?;
    Ok(Comparison {
        shape: class
            .iter()
            .map(|class| shape(class, &vocabulary))
            .collect(),
    })
}
