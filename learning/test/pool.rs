use crate::corpus::addition;
use crate::encoding::ATOM;
use crate::objective::Setting;
use crate::pool::admit;
use crate::problem::Failure;
use crate::task::Task;
use translation::vocabulary::Vocabulary;

#[test]
fn admission() {
    let setting = Setting::default();
    let task = addition(2);
    let pool = admit(Vec::new(), task.clone(), &setting).unwrap();
    let pool = admit(pool, task.clone(), &setting).unwrap();
    assert_eq!(pool.len(), 1);
    let crowded = Task {
        vocabulary: Vocabulary::alphabet(ATOM).unwrap(),
        ..task.clone()
    };
    assert!(matches!(
        admit(pool.clone(), crowded, &setting),
        Err(Failure::Vocabulary { .. })
    ));
    let full = Task {
        vocabulary: Vocabulary::alphabet(1 << u16::BITS).unwrap(),
        ..task
    };
    assert!(matches!(
        admit(pool, full, &setting),
        Err(Failure::Hidden { .. })
    ));
}
