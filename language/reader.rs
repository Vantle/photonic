#[derive(Clone, Copy)]
pub(crate) struct Read {
    pub site: usize,
    pub resource: usize,
}

pub(crate) struct Reader {
    pub rule: usize,
    pub read: Read,
    pub owner: usize,
}
