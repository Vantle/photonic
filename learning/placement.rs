use gpu::engine::Engine;
use gpu::failure::Failure;
use network::model::Model;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Placement {
    Automatic,
    Graphics,
    Processor,
}

impl Placement {
    pub(crate) fn engine(self, model: &Model) -> Result<Option<Engine>, Failure> {
        match self {
            Self::Processor => Ok(None),
            Self::Graphics => Engine::new(model).map(Some),
            Self::Automatic => Ok(Engine::new(model).ok()),
        }
    }
}
