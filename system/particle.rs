#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Particle<Concept> {
    concept: Vec<Concept>,
}

impl<Concept> Particle<Concept> {
    pub fn concept(&self) -> &[Concept] {
        &self.concept
    }
}

impl<Concept: Ord> Particle<Concept> {
    pub fn new(concept: impl IntoIterator<Item = Concept>) -> Self {
        let mut concept = concept.into_iter().collect::<Vec<_>>();
        concept.sort_unstable();
        Self { concept }
    }
}

impl<Concept: Clone + Ord> Particle<Concept> {
    pub fn remainder(&self, pattern: &Self) -> Option<Self> {
        if pattern.concept.len() > self.concept.len() {
            return None;
        }
        let mut matched = 0;
        let mut concept = Vec::with_capacity(self.concept.len());
        for value in &self.concept {
            let Some(expected) = pattern.concept.get(matched) else {
                concept.push(value.clone());
                continue;
            };
            match value.cmp(expected) {
                std::cmp::Ordering::Less => concept.push(value.clone()),
                std::cmp::Ordering::Equal => matched += 1,
                std::cmp::Ordering::Greater => return None,
            }
        }
        (matched == pattern.concept.len()).then_some(Self { concept })
    }

    pub fn merge(&self, other: &Self) -> Self {
        let mut concept = Vec::with_capacity(self.concept.len() + other.concept.len());
        let mut left = self.concept.iter().peekable();
        let mut right = other.concept.iter().peekable();
        while let (Some(first), Some(second)) = (left.peek(), right.peek()) {
            if first <= second {
                concept.push(left.next().unwrap().clone());
            } else {
                concept.push(right.next().unwrap().clone());
            }
        }
        concept.extend(left.cloned());
        concept.extend(right.cloned());
        Self { concept }
    }
}

impl<Concept> IntoIterator for Particle<Concept> {
    type Item = Concept;
    type IntoIter = std::vec::IntoIter<Concept>;

    fn into_iter(self) -> Self::IntoIter {
        self.concept.into_iter()
    }
}
