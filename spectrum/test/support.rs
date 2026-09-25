use crate::budget::Budget;
use crate::exploration::{Exploration, Plan};
use crate::recording::Mode;

pub const ORIGINAL: &str = "And.True.False.Extra,
[True] Boolean,
[False] Boolean,
[And.Boolean.Boolean] (
    [True.True] True,
    [True.False] False,
    [False.False] False,
)
";

pub const RENAMED: &str = "And.True.False.Rest,
[True] Boolean,
[False] Boolean,
[And.Boolean.Boolean] (
    [True.True] True,
    [True.False] False,
    [False.False] False,
)
";

pub const BUG: &str = "And.True.False.Extra,
[True] Boolean,
[False] Boolean,
[And.Boolean.Boolean] (
    [True.True] True,
    [False] False,
)
";

pub const FIX: &str = "And.True.False.Extra,
[True] Boolean,
[False] Boolean,
[And.Boolean.Boolean] (
    [True.True] True,
    [False.Boolean] False,
)
";

pub fn explore(source: &str) -> Exploration {
    let program = photonic::lowering::parse(source).expect("the example lowers");
    Exploration::new(Plan::new(
        &program,
        Mode::Exhaustive,
        Budget::default(),
        None,
    ))
    .expect("the example explores")
}
