use code::atom::Atom;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Vocabulary {
    name: Vec<String>,
}

impl Vocabulary {
    pub fn new(name: Vec<String>) -> Self {
        Self { name }
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
