#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub(crate) enum Link {
    Context = 0,
    World = 1,
    Member = 2,
    Particle = 3,
    Parent = 4,
    Child = 5,
    Lexical = 6,
    Closure = 7,
    Held = 8,
    Holder = 9,
    Capture = 10,
    Reference = 11,
    Owned = 12,
    Owner = 13,
}

impl Link {
    pub fn reverse(self) -> Self {
        match self {
            Self::Context => Self::World,
            Self::World => Self::Context,
            Self::Member => Self::Particle,
            Self::Particle => Self::Member,
            Self::Parent => Self::Child,
            Self::Child => Self::Parent,
            Self::Lexical => Self::Closure,
            Self::Closure => Self::Lexical,
            Self::Held => Self::Holder,
            Self::Holder => Self::Held,
            Self::Capture => Self::Reference,
            Self::Reference => Self::Capture,
            Self::Owned => Self::Owner,
            Self::Owner => Self::Owned,
        }
    }
}
