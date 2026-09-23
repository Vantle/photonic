#[derive(Debug)]
pub enum Rejection {
    Character,
    Spacing,
}

impl std::fmt::Display for Rejection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Character => "Use base-three digits 0, 1, 2, operators + − × ÷, and parentheses.",
            Self::Spacing => "Keep each numeral together; spaces separate operators, not digits.",
        })
    }
}

impl std::error::Error for Rejection {}

pub fn token(input: &str) -> Result<Vec<String>, Rejection> {
    let mut token = Vec::new();
    let mut gap = false;
    for character in input.chars() {
        if character.is_whitespace() {
            gap = true;
            continue;
        }
        if gap
            && matches!(character, '0'..='2')
            && token
                .last()
                .is_some_and(|value: &String| value.starts_with("([Digit]"))
        {
            return Err(Rejection::Spacing);
        }
        gap = false;
        token.push(match character {
            '0'..='2' => format!("([Digit] {character})"),
            '+' => "Add".into(),
            '-' | '−' => "Subtract".into(),
            '*' | '×' => "Multiply".into(),
            '/' | '÷' => "Divide".into(),
            '(' => "Open".into(),
            ')' => "Close".into(),
            _ => return Err(Rejection::Character),
        });
    }
    Ok(token)
}

pub fn stack(token: &[String], request: &str) -> Option<String> {
    let (last, rest) = token.split_last()?;
    let mut source = format!("Push.{last}.Zero, Stage.1\n");
    for (index, value) in (1..).zip(rest.iter().rev()) {
        source.push_str(&format!(
            "[Built, {}] (Push.{value}) (Forget.Stage.{})\n",
            stage(index),
            index + 1
        ));
    }
    source.push_str(&format!("[Built, {}] {request}\n", stage(token.len())));
    Some(source)
}

fn stage(index: usize) -> String {
    if index == 1 {
        return "Stage.1".into();
    }
    format!("Clean.Stage.{index}")
}
