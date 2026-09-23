use std::sync::Arc;

#[derive(Clone, Default)]
pub(crate) struct Revision(Arc<()>);

impl PartialEq for Revision {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for Revision {}
