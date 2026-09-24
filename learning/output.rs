use learning::archive::Improvement;
use learning::session::{Event, Origin};
use std::io::Write;

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
        "discovery {}: cost {before} -> {:.3} ({saving:+.1}% vs baseline {:.3}), size {}, time {:.3}, {}{}",
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
        if improvement.record.general {
            ""
        } else {
            ", fails holdout"
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
            report.fit,
            report.judge,
            report.risk * 100.0,
            report.reward,
            report.correct * 100.0,
            report.improvement,
        )),
        Event::Discovery(improvement) => line(&discovery(&improvement)),
    }
}
