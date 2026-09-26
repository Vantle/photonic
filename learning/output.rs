use learning::archive::{Coverage, Improvement};
use learning::attempt::{self, Count, Cover};
use learning::export::source;
use learning::guide::Guidance;
use learning::objective::TOLERANCE;
use learning::session::{Event, Origin};
use learning::solution::Solution;
use learning::task::Task;
use std::io::Write;
use std::time::Duration;

pub fn line(text: &str) {
    let mut output = std::io::stdout().lock();
    let _ = writeln!(output, "{text}");
    let _ = output.flush();
}

pub fn discovery(improvement: &Improvement) -> String {
    let saving = 100.0 * (improvement.baseline - improvement.after) / improvement.baseline;
    let before = improvement
        .before
        .map_or_else(|| "none".to_owned(), |cost| format!("{cost:.3}"));
    format!(
        "discovery {}: cost {before} -> {:.3} ({saving:+.1}% vs baseline {:.3}), size {}, time {:.3}, {}",
        improvement.task,
        improvement.after,
        improvement.baseline,
        improvement.record.size,
        improvement.record.time,
        if improvement.record.verified {
            "exhaustive"
        } else {
            "sampled"
        },
    )
}

pub fn observe(event: Event) {
    match event {
        Event::Start {
            parameter,
            origin,
            inference,
            training,
            worker,
        } => line(&format!(
            "network: {parameter} parameters, {}; inference on {inference}, training on {}; {worker} self-play threads",
            match origin {
                Origin::Fresh => "newly initialized".to_owned(),
                Origin::Restored => "restored from the learner home".to_owned(),
                Origin::Grown { from } =>
                    format!("grown from {from} parameters, keeping what it learned"),
                Origin::Kept(failure) => format!("restored at its saved shape because {failure}"),
            },
            training.as_deref().unwrap_or("nothing (frozen)")
        )),
        Event::Lesson { count } => line(&format!(
            "lessons: {count} best known programs demonstrated from an empty program"
        )),
        Event::Report(report) => line(&format!(
            "{:>7.0}s episodes {} decisions {} evaluations {} judged {} skipped {} inferences {} samples {} | train {} policy {:.3} value {:.4} entropy {:.2} judge {:.3} | judge error {:.3} lost {:.1}% | return {:+.4} correct {:.0}% | discoveries {}",
            report.elapsed,
            report.episode,
            report.step,
            report.evaluation,
            report.inferred,
            report.skipped,
            report.inference,
            report.sample,
            report.train,
            report.policy,
            report.value,
            report.entropy,
            report.judge,
            report.fit,
            report.risk * 100.0,
            report.reward,
            report.correct * 100.0,
            report.improvement,
        )),
        Event::Discovery(improvement) => line(&discovery(&improvement)),
    }
}

fn scope(coverage: Coverage) -> &'static str {
    match coverage {
        Coverage::Whole => "",
        Coverage::Flat => " among flat programs",
    }
}

fn describe(
    task: &Task,
    known: f64,
    solution: &Solution,
    elapsed: Duration,
    coverage: Coverage,
) -> String {
    let examined = format!(
        "{} programs examined in {:.1}s",
        solution.examined,
        elapsed.as_secs_f64()
    );
    let reached = solution.size.map_or_else(
        || "before any size was complete".to_owned(),
        |size| format!("every program up to size {size}"),
    );
    let found = solution.cost <= known + TOLERANCE;
    let within = scope(coverage);
    match (solution.proven, found) {
        (true, true) if solution.complete => format!(
            "{}: optimal cost {:.3}{within}, reached by {} of {examined}",
            task.name,
            solution.cost,
            solution.optimal.len()
        ),
        (true, true) => format!(
            "{}: optimal cost {:.3}{within}, reached by {} of {examined}; programs above size {} may tie it",
            task.name,
            solution.cost,
            solution.optimal.len(),
            solution.size.unwrap_or(0)
        ),
        (true, false) if solution.complete => format!(
            "{}: no flat program is as cheap as the known {known:.3}; {examined}",
            task.name
        ),
        (true, false) => format!(
            "{}: no flat program is cheaper than the known {known:.3}; programs above size {} may tie it; {examined}",
            task.name,
            solution.size.unwrap_or(0)
        ),
        (false, _) if solution.cost.is_finite() => format!(
            "{}: best cost {:.3} after {reached}; a cheaper program costs at least {:.3}; {examined}",
            task.name,
            solution.cost,
            solution.gap()
        ),
        (false, _) => format!("{}: nothing correct in {reached}; {examined}", task.name),
    }
}

fn guided(task: &Task, guidance: &Guidance) -> String {
    match (&guidance.best, guidance.first) {
        (Some((_, evaluation)), Some(first)) => format!(
            "{}: guided search found cost {:.3} at size {} among {} programs, the first correct one after {first}",
            task.name,
            evaluation.cost,
            evaluation.size.total(),
            guidance.expanded
        ),
        _ => format!(
            "{}: guided search found nothing correct among {} programs",
            task.name, guidance.expanded
        ),
    }
}

pub fn proof(count: &Count) -> String {
    if count.flat == 0 {
        return format!("{} proven optimal", count.proven);
    }
    format!(
        "{} proven optimal, {} among flat programs",
        count.proven, count.flat
    )
}

fn reason(cover: Cover) -> String {
    match cover.size {
        Some(size) => format!(
            "every program above size {size} costs at least {:.3}",
            cover.gap
        ),
        None => format!("every correct program costs at least {:.3}", cover.gap),
    }
}

pub fn report(event: attempt::Event<'_>, show: usize) {
    match event {
        attempt::Event::Skip(failure) => line(&format!("skipped: {failure}")),
        attempt::Event::Unguided(home) => line(&format!(
            "guided search skipped: no trained network is saved in {} yet",
            home.display()
        )),
        attempt::Event::Refusal { task, failure } => line(&format!("{}: {failure}", task.name)),
        attempt::Event::Exhaustion {
            task,
            known,
            solution,
            elapsed,
            coverage,
        } => line(&describe(task, known, solution, elapsed, coverage)),
        attempt::Event::Search { task, guidance } => line(&guided(task, guidance)),
        attempt::Event::Proof {
            task,
            cover,
            coverage,
        } => line(&format!(
            "{}: optimal{}, because {}",
            task.name,
            scope(coverage),
            reason(cover)
        )),
        attempt::Event::Finding { task, candidate } => {
            for (program, evaluation) in candidate.iter().take(show) {
                line(&format!(
                    "size {}, work {:.2}, span {:.2}\n{}",
                    evaluation.size.total(),
                    evaluation.work,
                    evaluation.span,
                    source(task, program)
                ));
            }
        }
        attempt::Event::Discovery(improvement) => line(&discovery(improvement)),
        attempt::Event::Holdout { task, passed } => line(&format!(
            "{}: {} {} held-out tests",
            task.name,
            if passed { "passes" } else { "fails" },
            task.holdout.len()
        )),
    }
}
