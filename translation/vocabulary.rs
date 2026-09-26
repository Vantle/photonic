use crate::failure::Failure;
use code::atom::Atom;
use indexmap::IndexSet;
use serde::{Deserialize, Serialize};

const LIMIT: usize = 1 << u16::BITS;

pub fn letter(index: usize) -> String {
    let mut rest = index + 1;
    let mut letter = Vec::new();
    while rest > 0 {
        rest -= 1;
        letter.push(char::from(b'A' + (rest % 26) as u8));
        rest /= 26;
    }
    letter.iter().rev().collect()
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(try_from = "Vec<String>", into = "Vec<String>")]
pub struct Vocabulary {
    name: IndexSet<String>,
}

impl TryFrom<Vec<String>> for Vocabulary {
    type Error = Failure;

    fn try_from(list: Vec<String>) -> Result<Self, Failure> {
        let mut vocabulary = Self::default();
        for name in list {
            if vocabulary.find(&name).is_some() {
                return Err(Failure::Duplicate { name });
            }
            vocabulary.intern(&name)?;
        }
        Ok(vocabulary)
    }
}

impl From<Vocabulary> for Vec<String> {
    fn from(vocabulary: Vocabulary) -> Self {
        vocabulary.name.into_iter().collect()
    }
}

impl Vocabulary {
    pub fn alphabet(count: usize) -> Result<Self, Failure> {
        Self::try_from((0..count).map(letter).collect::<Vec<_>>())
    }

    pub fn intern(&mut self, name: &str) -> Result<Atom, Failure> {
        if let Some(atom) = self.find(name) {
            return Ok(atom);
        }
        if name.is_empty() || name.chars().any(frontend::parser::separator) {
            return Err(Failure::Name {
                name: name.to_owned(),
            });
        }
        if self.name.len() == LIMIT {
            return Err(Failure::Vocabulary { limit: LIMIT });
        }
        let (index, _) = self.name.insert_full(name.to_owned());
        Ok(Atom(index as u16))
    }

    pub fn find(&self, name: &str) -> Option<Atom> {
        self.name.get_index_of(name).map(|index| Atom(index as u16))
    }

    pub fn name(&self, atom: Atom) -> &str {
        &self.name[atom.index()]
    }

    pub fn len(&self) -> usize {
        self.name.len()
    }

    pub fn is_empty(&self) -> bool {
        self.name.is_empty()
    }

    pub fn atom(&self) -> impl Iterator<Item = Atom> + '_ {
        (0..self.name.len()).map(|index| Atom(index as u16))
    }
}
