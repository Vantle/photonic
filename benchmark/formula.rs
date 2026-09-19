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
        let next = index + 1;
        source.push_str(&format!("[Built,Stage.{index}] (Push.{value}) (Forget.Stage.{next})\n[Clean.Stage.{next}] Stage.{next}\n"));
    }
    source.push_str(&format!("[Built,Stage.{}] (Function.Expression) (Forget.Inspect.0)\n[Clean.Inspect.0] Inspect.0\n[Return.Expression.Number.Positive,Inspect.0] (Read) (Forget.Inspect.1)\n[Clean.Inspect.1] Inspect.1\n", token.len()));
    for (index, digit) in expected.chars().rev().enumerate() {
        let current = index + 1;
        let next = current + 1;
        if current == expected.len() {
            source.push_str(&format!(
                "[Yield.([Digit] {digit}).Zero,Inspect.{current}] Done.Zero\n"
            ));
        } else {
            source.push_str(&format!("[Yield.([Digit] {digit}),Inspect.{current}] (Read) (Forget.Inspect.{next})\n[Clean.Inspect.{next}] Inspect.{next}\n"));
        }
    }
    source
}
