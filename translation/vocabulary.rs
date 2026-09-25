use crate::failure::Failure;
use code::atom::Atom;
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

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(try_from = "Vec<String>", into = "Vec<String>")]
pub struct Vocabulary {
    name: Vec<String>,
}

impl TryFrom<Vec<String>> for Vocabulary {
    type Error = Failure;

    fn try_from(name: Vec<String>) -> Result<Self, Failure> {
        if name.len() > LIMIT {
            return Err(Failure::Vocabulary { limit: LIMIT });
        }
        Ok(Self { name })
    }
}

impl From<Vocabulary> for Vec<String> {
    fn from(vocabulary: Vocabulary) -> Self {
        vocabulary.name
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
        if self.name.len() == LIMIT {
            return Err(Failure::Vocabulary { limit: LIMIT });
        }
        self.name.push(name.to_owned());
        Ok(Atom((self.name.len() - 1) as u16))
    }

    pub fn find(&self, name: &str) -> Option<Atom> {
        self.name
            .iter()
            .position(|entry| entry == name)
            .map(|index| Atom(index as u16))
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
