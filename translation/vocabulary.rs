use code::atom::Atom;
use serde::{Deserialize, Serialize};

fn letter(index: usize) -> String {
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
#[serde(transparent)]
pub struct Vocabulary {
    name: Vec<String>,
}

impl Vocabulary {
    pub fn new(name: Vec<String>) -> Self {
        Self { name }
    }

    pub fn alphabet(count: usize) -> Self {
        Self::new((0..count).map(letter).collect())
    }

    pub fn intern(&mut self, name: &str) -> Atom {
        if let Some(atom) = self.find(name) {
            return atom;
        }
        self.name.push(name.to_owned());
        Atom((self.name.len() - 1) as u16)
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
