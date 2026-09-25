#[derive(Clone)]
pub(crate) struct Address {
    role: &'static str,
    position: Option<usize>,
}

impl Address {
    pub(crate) fn scalar(role: &'static str) -> Self {
        Self {
            role,
            position: None,
        }
    }

    pub(crate) fn at(role: &'static str, position: usize) -> Self {
        Self {
            role,
            position: Some(position),
        }
    }

    pub(crate) fn rule(&self, value: u8) -> String {
        let position = self
            .position
            .map(|value| format!(".{value}"))
            .unwrap_or_default();
        format!("[{}{position}] {value}", self.role)
    }

    pub(crate) fn field(&self, value: u8) -> String {
        format!("({})", self.rule(value))
    }
}
