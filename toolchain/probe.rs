use std::process::ExitCode;

fn main() -> ExitCode {
    let expected = (0..20)
        .map(|value| value.to_string())
        .chain(["", "space value", "雪", "tail"].map(str::to_owned))
        .collect::<Vec<_>>();
    assert_eq!(std::env::args().skip(1).collect::<Vec<_>>(), expected);
    ExitCode::from(7)
}
