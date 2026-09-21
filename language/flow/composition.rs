use super::union::Union;
use super::{Flow, Place};
use crate::basis::Set;

#[derive(Default)]
pub(super) struct Composition {
    resource: Union<Place>,
    context: Union<usize>,
}

impl Composition {
    pub fn compose(&mut self, parent: &Flow, event: &Flow) -> Flow {
        #[cfg(feature = "measurement")]
        let _measurement = crate::measurement::profile::Scope::new(
            crate::measurement::profile::Phase::Composition,
        );
        Flow {
            resource: event
                .resource
                .iter()
                .map(|(&place, source)| {
                    if source.len() == 1 {
                        return (place, parent.resource[source.first().unwrap()].clone());
                    }
                    (
                        place,
                        self.resource.select(source, || {
                            source
                                .iter()
                                .flat_map(|key| parent.resource[key].iter().copied())
                                .collect::<Set<_>>()
                        }),
                    )
                })
                .collect(),
            context: event
                .context
                .iter()
                .map(|source| {
                    if source.len() == 1 {
                        return parent.context[*source.first().unwrap()].clone();
                    }
                    self.context.select(source, || {
                        source
                            .iter()
                            .flat_map(|&index| parent.context[index].iter().copied())
                            .collect::<Set<_>>()
                    })
                })
                .collect(),
            frame: event
                .frame
                .iter()
                .map(|index| index.and_then(|index| parent.frame[index]))
                .collect(),
        }
    }

    pub fn retained(&self) -> usize {
        self.resource.retained() + self.context.retained() + 1
    }
}
