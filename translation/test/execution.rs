use crate::execution::{explore, walk};
use crate::lift;
use crate::vocabulary::Vocabulary;
use code::configuration::Configuration;
use code::program::Program;
use photonic::execution::Bound;
use photonic::runtime::Limit;
use std::time::Instant;

fn chain(length: usize) -> (Program, Configuration, Vocabulary) {
    let mut source = String::from("Stage0,\n");
    for stage in 0..length {
        source.push_str(&format!("[Stage{stage}] Stage{},\n", stage + 1));
    }
    let mut vocabulary = Vocabulary::default();
    let (program, initial) = lift::program(
        &frontend::lowering::parse(&source).unwrap(),
        &mut vocabulary,
    )
    .unwrap();
    (program, initial, vocabulary)
}

#[test]
fn deadline() {
    let (program, initial, vocabulary) = chain(40);
    let open = Bound::default();
    let complete = explore(
        &program,
        &initial,
        &vocabulary,
        Limit::default(),
        open,
        |_| false,
    );
    assert!(complete.complete());
    assert_eq!(complete.state, 41);
    let past = Bound {
        deadline: Some(Instant::now()),
        ..open
    };
    let stopped = explore(
        &program,
        &initial,
        &vocabulary,
        Limit::default(),
        past,
        |_| false,
    );
    assert!(stopped.overflow);
    assert_eq!(stopped.state, 1);
    let finished = walk(
        &program,
        &initial,
        &vocabulary,
        Limit::default(),
        open,
        |_| 0,
    );
    assert!(!finished.overflow);
    assert_eq!(finished.step, 40);
    let interrupted = walk(
        &program,
        &initial,
        &vocabulary,
        Limit::default(),
        past,
        |_| 0,
    );
    assert!(interrupted.overflow);
    assert_eq!(interrupted.step, 0);
}
