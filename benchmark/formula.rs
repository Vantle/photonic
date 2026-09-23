pub fn source(input: &str, expected: &str) -> String {
    let token = input
        .chars()
        .rev()
        .map(|value| match value {
            '+' => "Add".to_owned(),
            '*' => "Multiply".to_owned(),
            '-' => "Subtract".to_owned(),
            '/' => "Divide".to_owned(),
            '(' => "Open".to_owned(),
            ')' => "Close".to_owned(),
            digit => format!("([Digit] {digit})"),
        })
        .collect::<Vec<_>>();
    let mut source = format!("Push.{}.Zero, Stage.1\n", token[0]);
    for (index, value) in token.iter().enumerate().skip(1) {
        source.push_str(&format!(
            "[Built, {}] (Push.{value}) (Forget.Stage.{})\n",
            stage(index),
            index + 1
        ));
    }
    source.push_str(&format!(
        "[Built, {}] (Function.Expression.Evaluate) (Forget.Inspect.0)\n[Return.Expression.Evaluate.Positive, Clean.Inspect.0] (Read) (Forget.Inspect.1)\n",
        stage(token.len())
    ));
    for (index, digit) in expected.chars().rev().enumerate() {
        let current = index + 1;
        if current == expected.len() {
            source.push_str(&format!(
                "[Yield.([Digit] {digit}).Zero, Clean.Inspect.{current}] Done.Zero\n"
            ));
        } else {
            source.push_str(&format!(
                "[Yield.([Digit] {digit}), Clean.Inspect.{current}] (Read) (Forget.Inspect.{})\n",
                current + 1
            ));
        }
    }
    source
}

fn stage(index: usize) -> String {
    if index == 1 {
        return "Stage.1".into();
    }
    format!("Clean.Stage.{index}")
}
