use crate::atom::Atom;
use crate::value::Value;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(from = "Vec<Value>", into = "Vec<Value>")]
pub struct Particle {
    value: Vec<Value>,
}

impl From<Vec<Value>> for Particle {
    fn from(mut value: Vec<Value>) -> Self {
        value.sort_unstable();
        Self { value }
    }
}

impl From<Particle> for Vec<Value> {
    fn from(particle: Particle) -> Self {
        particle.value
    }
}

impl Particle {
    pub fn atom(atom: &[Atom]) -> Self {
        Self::from(
            atom.iter()
                .map(|&atom| Value::Atom(atom))
                .collect::<Vec<_>>(),
        )
    }

    pub fn value(&self) -> &[Value] {
        &self.value
    }

    pub fn len(&self) -> usize {
        self.value.len()
    }

    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    pub fn flat(&self) -> Option<Vec<Atom>> {
        self.value.iter().map(Value::atom).collect()
    }

    pub fn insert(&self, value: Value) -> Self {
        let mut result = self.value.clone();
        let position = result.partition_point(|entry| *entry <= value);
        result.insert(position, value);
        Self { value: result }
    }

    pub fn remove(&self, value: &Value) -> Option<Self> {
        let position = self.value.binary_search(value).ok()?;
        let mut result = self.value.clone();
        result.remove(position);
        Some(Self { value: result })
    }

    pub fn difference(&self, other: &Self) -> usize {
        let mut left = self.value.iter().peekable();
        let mut right = other.value.iter().peekable();
        let mut count = 0;
        loop {
            match (left.peek(), right.peek()) {
                (Some(first), Some(second)) => match first.cmp(second) {
                    std::cmp::Ordering::Equal => {
                        left.next();
                        right.next();
                    }
                    std::cmp::Ordering::Less => {
                        left.next();
                        count += 1;
                    }
                    std::cmp::Ordering::Greater => {
                        right.next();
                        count += 1;
                    }
                },
                (Some(_), None) => {
                    left.next();
                    count += 1;
                }
                (None, Some(_)) => {
                    right.next();
                    count += 1;
                }
                (None, None) => return count,
            }
        }
    }
}
