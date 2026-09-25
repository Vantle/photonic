use crate::exploration::Exploration;
use crate::failure::{Code, Failure};
use std::fmt;
use std::str::FromStr;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Handle {
    Rule(usize),
    Configuration(usize),
    Event(usize),
    Coherence(usize, usize),
    Occurrence(usize, usize),
    Frame(usize, usize),
}

fn number(text: &str, prefix: char) -> Option<usize> {
    let digit = text.strip_prefix(prefix)?;
    if digit.is_empty() || !digit.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    digit.parse().ok()
}

impl FromStr for Handle {
    type Err = Failure;

    fn from_str(text: &str) -> Result<Self, Failure> {
        let invalid = || {
            Failure::new(
                Code::Handle,
                format!("{text} is not a handle; write r2, s11, e12, s11.c0, s11.o1 or s10.f1"),
            )
        };
        let text = text.trim();
        if let Some((configuration, inner)) = text.split_once('.') {
            let configuration = number(configuration, 's').ok_or_else(invalid)?;
            return number(inner, 'c')
                .map(|index| Self::Coherence(configuration, index))
                .or_else(|| number(inner, 'o').map(|id| Self::Occurrence(configuration, id)))
                .or_else(|| number(inner, 'f').map(|index| Self::Frame(configuration, index)))
                .ok_or_else(invalid);
        }
        number(text, 'r')
            .map(Self::Rule)
            .or_else(|| number(text, 's').map(Self::Configuration))
            .or_else(|| number(text, 'e').map(Self::Event))
            .ok_or_else(invalid)
    }
}

impl fmt::Display for Handle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rule(index) => write!(formatter, "r{index}"),
            Self::Configuration(index) => write!(formatter, "s{index}"),
            Self::Event(index) => write!(formatter, "e{index}"),
            Self::Coherence(configuration, index) => write!(formatter, "s{configuration}.c{index}"),
            Self::Occurrence(configuration, id) => write!(formatter, "s{configuration}.o{id}"),
            Self::Frame(configuration, index) => write!(formatter, "s{configuration}.f{index}"),
        }
    }
}

impl Handle {
    pub(crate) fn check(self, exploration: &Exploration) -> Result<Self, Failure> {
        let configuration = |index: usize| exploration.configuration.get(index);
        let found = match self {
            Self::Rule(index) => exploration.rule.get(index).map(|_| ()),
            Self::Configuration(index) => configuration(index).map(|_| ()),
            Self::Event(index) => exploration.event.get(index).map(|_| ()),
            Self::Coherence(index, world) => {
                configuration(index).and_then(|entry| entry.coherence.get(world).map(|_| ()))
            }
            Self::Occurrence(index, id) => exploration.find(index, id).map(|_| ()),
            Self::Frame(index, frame) => {
                configuration(index).and_then(|entry| entry.frame.get(frame).map(|_| ()))
            }
        };
        if found.is_some() {
            return Ok(self);
        }
        let what = match self {
            Self::Rule(_) => "rule",
            Self::Configuration(_) => "configuration",
            Self::Event(_) => "event",
            Self::Coherence(..) => "coherence",
            Self::Occurrence(..) => "occurrence",
            Self::Frame(..) => "frame",
        };
        Err(Failure::new(
            Code::Handle,
            format!(
                "{self} names no {what} in exploration {}",
                exploration.name()
            ),
        ))
    }
}
