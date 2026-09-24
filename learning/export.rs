use crate::objective::Setting;
use crate::task::Task;
use code::program::Program;
use serde::Serialize;
use translation::{emit, execution, text};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub struct Verification {
    pub total: usize,
    pub kernel: usize,
    pub prism: usize,
    pub expressible: usize,
}

pub fn source(task: &Task, program: &Program) -> String {
    text::program(program, &task.vocabulary)
}

pub fn verify(task: &Task, program: &Program, setting: &Setting, budget: usize) -> Verification {
    let mut verification = Verification {
        total: task.example.len() + task.holdout.len(),
        ..Verification::default()
    };
    let key = setting.limit.individualization;
    for example in task.example.iter().chain(&task.holdout) {
        let exploration = execution::explore(
            program,
            &example.input,
            &task.vocabulary,
            setting.admission,
            setting.bound,
            |_| false,
        );
        if exploration.complete()
            && exploration.terminal.len() == 1
            && exploration.terminal[0].key(key) == example.output.key(key)
        {
            verification.kernel += 1;
        }
        if example.output.shared() {
            continue;
        }
        verification.expressible += 1;
        let mut search = photonic::prism::Search::new(
            emit::program(program, &example.input, &task.vocabulary),
            emit::program(program, &example.output.configuration(), &task.vocabulary),
        );
        search.run(budget, Some(setting.admission));
        if search.verdict().outcome == photonic::prism::Outcome::Reached {
            verification.prism += 1;
        }
    }
    verification
}
