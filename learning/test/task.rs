use crate::corpus::addition;
use crate::task::Task;
use translation::failure::Failure;
use translation::vocabulary::Vocabulary;

#[test]
fn conceal() {
    let task = addition(2);
    let count = task.vocabulary.len();
    assert_eq!(task.clone().conceal(2).unwrap().vocabulary.len(), count + 2);
    let full = Task {
        vocabulary: Vocabulary::alphabet(1 << u16::BITS).unwrap(),
        ..task
    };
    assert!(matches!(full.conceal(1), Err(Failure::Vocabulary { .. })));
}
