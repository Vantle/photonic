use crate::index::Index;
use crate::term::Term;
#[cfg(test)]
use std::sync::Arc;

pub(super) enum Query {
    Planned(crate::plan::Context),
    #[cfg(test)]
    Direct {
        pattern: Arc<Vec<Vec<Term>>>,
        group: Vec<usize>,
    },
}

impl Query {
    #[cfg(test)]
    pub fn direct(pattern: Arc<Vec<Vec<Term>>>) -> Self {
        Self::Direct {
            group: crate::partition::classify(&pattern),
            pattern,
        }
    }

    pub fn count(&self) -> usize {
        match self {
            Self::Planned(context) => context.count(),
            #[cfg(test)]
            Self::Direct { pattern, .. } => pattern.len(),
        }
    }

    pub fn width(&self, position: usize) -> usize {
        match self {
            Self::Planned(context) => context.width(position),
            #[cfg(test)]
            Self::Direct { pattern, .. } => pattern[position].len(),
        }
    }

    pub fn group(&self) -> &[usize] {
        match self {
            Self::Planned(context) => context.group(),
            #[cfg(test)]
            Self::Direct { group, .. } => group,
        }
    }

    pub fn pattern(&self, position: usize) -> Vec<Term> {
        match self {
            Self::Planned(context) => context.pattern(position).collect(),
            #[cfg(test)]
            Self::Direct { pattern, .. } => pattern[position].clone(),
        }
    }

    pub fn affected(&self, position: usize, index: &Index, frame: usize) -> bool {
        match self {
            Self::Planned(context) => context.affected(position, index, frame),
            #[cfg(test)]
            Self::Direct { .. } => true,
        }
    }

    pub fn matches(&self, position: usize, index: &Index, site: usize) -> bool {
        match self {
            Self::Planned(context) => context.matches(position, index, site),
            #[cfg(test)]
            Self::Direct { pattern, .. } => index.matches(site, pattern[position].iter().cloned()),
        }
    }

    pub fn possible(&self, index: &Index, frame: usize) -> bool {
        (0..self.count()).all(|position| match self {
            Self::Planned(context) => context.possible(position, index, frame),
            #[cfg(test)]
            Self::Direct { pattern, .. } => {
                index.possible(pattern[position].iter().cloned(), frame)
            }
        })
    }

    pub fn candidate(
        &self,
        position: usize,
        index: &Index,
        frame: usize,
        store: Option<&crate::candidate::Store>,
    ) -> crate::candidate::Domain {
        let site = match self {
            Self::Planned(context) => {
                if let Some(store) = store {
                    return store.select(crate::candidate::Request {
                        pattern: context.pattern(position),
                        index,
                        frame,
                    });
                }
                context.candidate(position, index, frame)
            }
            #[cfg(test)]
            Self::Direct { pattern, .. } => {
                if let Some(store) = store {
                    return store.select(crate::candidate::Request {
                        pattern: pattern[position].iter().cloned(),
                        index,
                        frame,
                    });
                }
                index.candidate(pattern[position].iter().cloned(), frame)
            }
        };
        crate::candidate::Domain { site, node: None }
    }

    pub fn select(
        &self,
        position: usize,
        index: &Index,
        site: usize,
        shared: Option<&crate::preparation::Store>,
    ) -> crate::particle::Match {
        match self {
            Self::Planned(context) => context.select(position, index, site, shared),
            #[cfg(test)]
            Self::Direct { pattern, .. } => crate::particle::Match::new(
                &pattern[position],
                &index.particle(site, pattern[position].iter().cloned()),
            ),
        }
    }

    pub fn retained(&self) -> usize {
        (0..self.count()).map(|position| self.width(position)).sum()
    }
}
