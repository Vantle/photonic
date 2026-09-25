use smallvec::SmallVec;

#[derive(Eq, Hash, PartialEq)]
struct Influence {
    position: usize,
    site: usize,
}

#[derive(Eq, Hash, PartialEq)]
pub(super) struct Dependency {
    site: usize,
    binding: SmallVec<[Influence; 2]>,
}

impl Dependency {
    pub fn new(site: usize, binding: impl Iterator<Item = (usize, usize)>) -> Self {
        Self {
            site,
            binding: binding
                .map(|(position, site)| Influence { position, site })
                .collect(),
        }
    }

    pub fn site(&self) -> impl Iterator<Item = usize> + '_ {
        std::iter::once(self.site).chain(
            self.binding
                .iter()
                .map(|influence| influence.site)
                .filter(|&site| site != self.site),
        )
    }

    pub fn retained(&self) -> usize {
        2 + self.binding.len() * 2 + self.site().count() * 2
    }
}
