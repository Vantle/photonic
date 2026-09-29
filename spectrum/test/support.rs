use crate::budget::Budget;
use crate::exploration::{Exploration, Plan};
use crate::recording::{Engine, Mode};

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
    engine(source, Engine::Interpreter)
}

pub fn engine(source: &str, engine: Engine) -> Exploration {
    record(source, Mode::Exhaustive, engine)
}

pub fn plain(source: &str) -> Exploration {
    record(source, Mode::Plain, Engine::Laser)
}

fn record(source: &str, mode: Mode, engine: Engine) -> Exploration {
    let program = frontend::lowering::parse(source).expect("the example lowers");
    Exploration::new(Plan::new(&program, mode, engine, Budget::default(), None))
}
