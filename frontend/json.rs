use serde::de::{self, DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};

use crate::failure::Failure;
use crate::parser::DEPTH;
use crate::source;

// A program read as data keeps to what its text can write: an object is a program, a scope or a
// rule, an array is a particle, a string is an atom that text could spell, and nothing nests
// deeper than text can. Each part carries how many levels of text enclose it, and a particle how
// many enclose its rule values, which text writes in parentheses except in a rule's input; so the
// reader refuses a program too deep as soon as it meets it, before its own recursion could run out
// of stack.

#[derive(Clone, Copy, Eq, PartialEq)]
enum Role {
    Program,
    Library,
    Scope,
}

#[derive(Clone, Copy)]
struct Program {
    depth: usize,
    role: Role,
}

#[derive(Clone, Copy)]
struct Definition {
    depth: usize,
}

#[derive(Clone, Copy)]
struct Output {
    depth: usize,
}

#[derive(Clone, Copy)]
struct Particle {
    depth: usize,
}

#[derive(Clone, Copy)]
struct Value {
    depth: usize,
}

#[derive(Clone, Copy)]
struct List<Seed>(Seed);

#[derive(Clone, Copy)]
struct Refusal;

const LIBRARY: &str = "a library holds only rules";

pub(crate) fn program(text: &str) -> Result<source::Program, Failure> {
    read(text, Role::Program).map_err(|error| Failure::Json {
        message: message(&error),
        span: span(text, &error),
    })
}

// A library is read as a program first, so a malformed one is refused as JSON; one that lists a
// coherence or scope is read again only to find the first it lists.
pub(crate) fn library(text: &str) -> Result<source::Library, Failure> {
    let program = program(text)?;
    if program.initial.is_empty() && program.scope.is_empty() {
        return Ok(source::Library { rule: program.rule });
    }
    let span = read(text, Role::Library).map_or_else(|error| span(text, &error), |_| (0, 0).into());
    Err(Failure::Library { span })
}

// Editors on Windows begin a file with a byte order mark, which RFC 8259 lets a reader ignore.
fn read(text: &str, role: Role) -> Result<source::Program, serde_json::Error> {
    let body = text.strip_prefix('\u{FEFF}').unwrap_or(text);
    let mut deserializer = serde_json::Deserializer::from_str(body);
    deserializer.disable_recursion_limit();
    let program = Program { depth: 0, role }.deserialize(&mut deserializer)?;
    deserializer.end()?;
    Ok(program)
}

fn message(error: &serde_json::Error) -> String {
    let text = error.to_string();
    let place = format!(" at line {} column {}", error.line(), error.column());
    text.strip_suffix(&place)
        .map_or(text.clone(), str::to_owned)
}

fn span(text: &str, error: &serde_json::Error) -> miette::SourceSpan {
    let mark = text.len() - text.strip_prefix('\u{FEFF}').unwrap_or(text).len();
    let start = text[mark..]
        .split_inclusive('\n')
        .take(error.line().saturating_sub(1))
        .map(str::len)
        .sum::<usize>()
        + mark;
    let mut offset = (start + error.column().saturating_sub(1)).min(text.len());
    while !text.is_char_boundary(offset) {
        offset -= 1;
    }
    let length = text[offset..].chars().next().map_or(0, char::len_utf8);
    (offset, length).into()
}

fn nesting<Error: de::Error>(depth: usize) -> Result<(), Error> {
    if depth > DEPTH {
        return Err(Error::custom(format!(
            "this nests deeper than the {DEPTH} levels text can write"
        )));
    }
    Ok(())
}

impl<'de> serde::Deserialize<'de> for source::Program {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Program {
            depth: 0,
            role: Role::Program,
        }
        .deserialize(deserializer)
    }
}

impl<'de> DeserializeSeed<'de> for Program {
    type Value = source::Program;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Program {
    type Value = source::Program;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.role {
            Role::Scope => formatter.write_str("a scope, an object with initial, rule and scope"),
            Role::Program | Role::Library => {
                formatter.write_str("a program, an object with initial, rule and scope")
            }
        }
    }

    fn visit_map<Map: MapAccess<'de>>(self, mut map: Map) -> Result<Self::Value, Map::Error> {
        nesting(self.depth)?;
        let (mut initial, mut rule, mut scope) = (None, None, None);
        let inner = Self {
            depth: self.depth + 1,
            role: Role::Scope,
        };
        let particle = List(Particle {
            depth: self.depth + 1,
        });
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "initial" if initial.is_some() => {
                    return Err(de::Error::duplicate_field("initial"));
                }
                "rule" if rule.is_some() => return Err(de::Error::duplicate_field("rule")),
                "scope" if scope.is_some() => return Err(de::Error::duplicate_field("scope")),
                "initial" | "scope" if self.role == Role::Library => {
                    map.next_value_seed(List(Refusal))?;
                }
                "initial" => initial = Some(map.next_value_seed(particle)?),
                "rule" => {
                    rule = Some(map.next_value_seed(List(Definition { depth: self.depth }))?);
                }
                "scope" => scope = Some(map.next_value_seed(List(inner))?),
                other => {
                    return Err(de::Error::unknown_field(
                        other,
                        &["initial", "rule", "scope"],
                    ));
                }
            }
        }
        let program = source::Program {
            initial: initial.unwrap_or_default(),
            rule: rule.unwrap_or_default(),
            scope: scope.unwrap_or_default(),
        };
        if self.role != Role::Scope {
            return Ok(program);
        }
        if program.rule.is_empty() {
            return Err(de::Error::custom(
                "a scope lists a rule; a group without one is only its members",
            ));
        }
        if program.initial.is_empty() && program.scope.is_empty() {
            return Err(de::Error::custom(
                "a scope holds a coherence or a scope; write the empty coherence as []",
            ));
        }
        Ok(program)
    }
}

impl<'de> DeserializeSeed<'de> for Definition {
    type Value = source::Definition;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Definition {
    type Value = source::Definition;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a rule, an object with input and output")
    }

    fn visit_map<Map: MapAccess<'de>>(self, mut map: Map) -> Result<Self::Value, Map::Error> {
        nesting(self.depth + 1)?;
        let (mut input, mut output) = (None, None);
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "input" if input.is_some() => return Err(de::Error::duplicate_field("input")),
                "output" if output.is_some() => return Err(de::Error::duplicate_field("output")),
                "input" => {
                    input = Some(map.next_value_seed(List(Particle {
                        depth: self.depth + 1,
                    }))?);
                }
                "output" => {
                    output = Some(map.next_value_seed(List(Output { depth: self.depth }))?);
                }
                other => return Err(de::Error::unknown_field(other, &["input", "output"])),
            }
        }
        Ok(source::Definition {
            input: input.ok_or_else(|| de::Error::missing_field("input"))?,
            output: output.ok_or_else(|| de::Error::missing_field("output"))?,
        })
    }
}

impl<'de> DeserializeSeed<'de> for Output {
    type Value = source::Output;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Output {
    type Value = source::Output;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(
            "an output: a particle, written as an array, or a scope, written as an object",
        )
    }

    fn visit_seq<Sequence: SeqAccess<'de>>(
        self,
        sequence: Sequence,
    ) -> Result<Self::Value, Sequence::Error> {
        Particle {
            depth: self.depth + 1,
        }
        .visit_seq(sequence)
        .map(source::Output::Particle)
    }

    fn visit_map<Map: MapAccess<'de>>(self, map: Map) -> Result<Self::Value, Map::Error> {
        Program {
            depth: self.depth + 1,
            role: Role::Scope,
        }
        .visit_map(map)
        .map(source::Output::Scope)
    }
}

impl<'de> DeserializeSeed<'de> for Particle {
    type Value = Vec<source::Value>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Particle {
    type Value = Vec<source::Value>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a particle, an array of atoms and rule values")
    }

    fn visit_seq<Sequence: SeqAccess<'de>>(
        self,
        sequence: Sequence,
    ) -> Result<Self::Value, Sequence::Error> {
        List(Value { depth: self.depth }).visit_seq(sequence)
    }
}

impl<'de> DeserializeSeed<'de> for Value {
    type Value = source::Value;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Value {
    type Value = source::Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .write_str("a value: an atom, written as a string, or a rule, written as {\"rule\": …}")
    }

    fn visit_str<Error: de::Error>(self, atom: &str) -> Result<Self::Value, Error> {
        crate::atom::check(atom).map_err(Error::custom)?;
        Ok(source::Value::Atom(atom.to_owned()))
    }

    fn visit_map<Map: MapAccess<'de>>(self, mut map: Map) -> Result<Self::Value, Map::Error> {
        let Some(key) = map.next_key::<String>()? else {
            return Err(de::Error::missing_field("rule"));
        };
        if key != "rule" {
            return Err(de::Error::unknown_field(&key, &["rule"]));
        }
        let rule = map.next_value_seed(Definition { depth: self.depth })?;
        if let Some(key) = map.next_key::<String>()? {
            return Err(de::Error::unknown_field(&key, &[]));
        }
        Ok(source::Value::Rule {
            rule: Box::new(rule),
        })
    }
}

impl<'de, Seed: DeserializeSeed<'de> + Copy> DeserializeSeed<'de> for List<Seed> {
    type Value = Vec<Seed::Value>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de, Seed: DeserializeSeed<'de> + Copy> Visitor<'de> for List<Seed> {
    type Value = Vec<Seed::Value>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("an array")
    }

    fn visit_seq<Sequence: SeqAccess<'de>>(
        self,
        mut sequence: Sequence,
    ) -> Result<Self::Value, Sequence::Error> {
        let mut result = Vec::new();
        while let Some(item) = sequence.next_element_seed(self.0)? {
            result.push(item);
        }
        Ok(result)
    }
}

impl<'de> DeserializeSeed<'de> for Refusal {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Refusal {
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(LIBRARY)
    }

    fn visit_seq<Sequence: SeqAccess<'de>>(self, _: Sequence) -> Result<(), Sequence::Error> {
        Err(de::Error::custom(LIBRARY))
    }

    fn visit_map<Map: MapAccess<'de>>(self, _: Map) -> Result<(), Map::Error> {
        Err(de::Error::custom(LIBRARY))
    }
}
