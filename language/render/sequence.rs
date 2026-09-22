use serde::{Serialize, Serializer};

pub(crate) struct Sequence<Factory> {
    create: Factory,
}

impl<Factory> Sequence<Factory> {
    pub fn new(create: Factory) -> Self {
        Self { create }
    }
}

impl<Factory, Iterator> Serialize for Sequence<Factory>
where
    Factory: Fn() -> Iterator,
    Iterator: IntoIterator,
    Iterator::Item: Serialize,
{
    fn serialize<Output: Serializer>(
        &self,
        serializer: Output,
    ) -> Result<Output::Ok, Output::Error> {
        serializer.collect_seq((self.create)())
    }
}
