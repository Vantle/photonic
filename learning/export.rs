use crate::archive::Archive;
use crate::home::{self, Home};
use crate::objective::Setting;
use crate::task::Task;
use code::program::Program;
use translation::{emit, execution, text};

pub const BUDGET: usize = 2_000_000;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
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
            && exploration.terminal[0].same(&example.output, key)
        {
            verification.kernel += 1;
        }
        if example.output.shared() {
            continue;
        }
        verification.expressible += 1;
        let mut runtime = photonic::runtime::Runtime::new(&emit::program(
            program,
            &example.input,
            &task.vocabulary,
        ));
        runtime.run(budget, setting.admission);
        let target = emit::program(program, &example.output.configuration(), &task.vocabulary);
        if runtime.verdict(&target).outcome == photonic::prism::Outcome::Reached {
            verification.prism += 1;
        }
    }
    verification
}

pub fn write(home: &Home, pool: &[Task], archive: &Archive) -> Result<(), home::Failure> {
    for task in pool {
        let Some(record) = archive.best(&task.name) else {
            continue;
        };
        home.write(
            &format!("{}/{}.wave", home::PROGRAM, task.name),
            &source(task, &record.program),
        )?;
    }
    Ok(())
}
