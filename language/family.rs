pub fn dial(count: usize) -> String {
    let mut text = (1..=count)
        .map(|index| format!("Dial.D{index}.Zero, "))
        .collect::<String>();
    text.push_str("[Zero] One, [One] Two, [Two] Zero");
    text
}

pub fn task(count: usize) -> String {
    let mut text = (1..=count)
        .map(|index| format!("Task.T{index}.Pending, "))
        .collect::<String>();
    text.push_str("[Pending] Running, [Running] Done");
    text
}

pub fn diner(count: usize) -> String {
    let mut term = (1..=count)
        .map(|index| format!("Think.D{index}, Fork.F{index}"))
        .collect::<Vec<_>>();
    for index in 1..=count {
        let next = index % count + 1;
        term.push(format!(
            "[Think.D{index}, Fork.F{index}] Waiting.D{index}, \
             [Waiting.D{index}, Fork.F{next}] Eat.D{index}, \
             [Eat.D{index}] (Think.D{index}, Fork.F{index}, Fork.F{next})"
        ));
    }
    term.join(", ")
}
