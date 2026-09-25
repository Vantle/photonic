use crate::corpus::curated;
use crate::encoding::ATOM;
use crate::export::verify;
use crate::objective::{Setting, evaluate};
use crate::pool::initial;
use crate::problem::Problem;

#[test]
fn reference() {
    let setting = Setting::default();
    for task in curated() {
        let name = task.name.clone();
        let reference = evaluate(
            task.reference.as_ref().unwrap(),
            &task.example,
            &task.vocabulary,
            &setting.aim(task.goal),
        );
        assert!(reference.verified, "{name}");
        let problem = Problem::new(task, &setting, ATOM)
            .unwrap_or_else(|failure| panic!("{name}: {failure}"));
        assert!(problem.baseline > 0.0, "{name}");
    }
}

#[test]
fn kernel() {
    let setting = Setting::default();
    for task in curated()
        .into_iter()
        .filter(|task| task.example.len() + task.holdout.len() <= 32)
    {
        let result = verify(&task, task.reference.as_ref().unwrap(), &setting, 200_000);
        assert_eq!(result.kernel, result.total, "{}", task.name);
        assert_eq!(result.prism, result.expressible, "{}", task.name);
    }
}

#[test]
fn synthetic() {
    let setting = Setting::default();
    let pool = initial(6, 3, &setting).unwrap();
    assert_eq!(
        pool.iter()
            .filter(|task| task.name.starts_with("synthetic."))
            .count(),
        6
    );
    for task in pool {
        let name = task.name.clone();
        Problem::new(task, &setting, ATOM).unwrap_or_else(|failure| panic!("{name}: {failure}"));
    }
}
