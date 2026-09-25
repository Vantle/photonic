use crate::task::{Example, Task};
use code::configuration::Configuration;
use code::observation::Observation;
use code::program::Program;
use random::Generator;
use translation::lift;
use translation::vocabulary::Vocabulary;

const POSITION: [&str; 8] = [
    "First", "Second", "Third", "Fourth", "Fifth", "Sixth", "Seventh", "Eighth",
];

fn parse(text: &str, vocabulary: &mut Vocabulary) -> (Program, Configuration) {
    let source = photonic::lowering::parse(text)
        .unwrap_or_else(|failure| panic!("corpus source must parse: {failure}: {text}"));
    lift::program(&source, vocabulary)
        .unwrap_or_else(|failure| panic!("corpus source must lift: {failure}: {text}"))
}

fn task(name: &str, reference: &str, example: &[(String, String)]) -> Task {
    let mut vocabulary = Vocabulary::default();
    let (reference, _) = parse(reference, &mut vocabulary);
    let mut seen = Vec::new();
    let example = example
        .iter()
        .map(|(input, output)| Example {
            input: parse(input, &mut vocabulary).1,
            output: Observation::from(&parse(output, &mut vocabulary).1),
        })
        .filter(|example| {
            if seen.contains(&example.input) {
                return false;
            }
            seen.push(example.input.clone());
            true
        })
        .collect();
    Task {
        name: name.to_owned(),
        vocabulary,
        hidden: 0,
        example,
        holdout: Vec::new(),
        reference: Some(reference),
        goal: None,
    }
}

fn holdout(mut task: Task, count: usize) -> Task {
    let mut generator = Generator::new(task.example.len() as u64);
    generator.shuffle(&mut task.example);
    task.holdout = task.example.split_off(count.min(task.example.len()));
    task
}

fn truth(value: bool) -> &'static str {
    if value { "True" } else { "False" }
}

fn table(name: &str, arity: usize, operation: impl Fn(usize) -> bool) -> Task {
    let mut rule = String::new();
    let mut example = Vec::new();
    for positive in (0..=arity).rev() {
        let operand = (0..arity)
            .map(|index| truth(index < positive))
            .collect::<Vec<_>>()
            .join(".");
        let input = format!("{name}.{operand}");
        let output = truth(operation(positive)).to_owned();
        rule.push_str(&format!("[{input}] {output},\n"));
        example.push((input, output));
    }
    task(&format!("boolean.{}", name.to_lowercase()), &rule, &example)
}

pub fn boolean() -> Vec<Task> {
    vec![
        table("Not", 1, |positive| positive == 0),
        table("And", 2, |positive| positive == 2),
        table("Or", 2, |positive| positive > 0),
        table("Xor", 2, |positive| positive == 1),
        table("Nand", 2, |positive| positive < 2),
        table("Nor", 2, |positive| positive == 0),
        table("Equal", 2, |positive| positive != 1),
        table("Majority", 3, |positive| positive >= 2),
        table("Parity", 3, |positive| positive % 2 == 1),
    ]
}

fn assignment(length: usize, value: usize) -> Vec<Vec<usize>> {
    (0..value.pow(length as u32))
        .map(|mut code| {
            (0..length)
                .map(|_| {
                    let digit = code % value;
                    code /= value;
                    digit
                })
                .collect()
        })
        .collect()
}

fn placed(value: &[usize]) -> String {
    value
        .iter()
        .enumerate()
        .map(|(index, value)| format!("{}.{value}", POSITION[index]))
        .collect::<Vec<_>>()
        .join(", ")
}

fn exchange(left: usize, right: usize, value: usize) -> String {
    let mut rule = String::new();
    for high in 0..value {
        for low in 0..high {
            rule.push_str(&format!(
                "[{first}.{high}, {second}.{low}] ({first}.{low}, {second}.{high}),\n",
                first = POSITION[left],
                second = POSITION[right],
            ));
        }
    }
    rule
}

pub fn sort(length: usize, value: usize) -> Task {
    let reference = (0..length - 1)
        .map(|left| exchange(left, left + 1, value))
        .collect::<String>();
    let example = assignment(length, value)
        .into_iter()
        .map(|input| {
            let mut output = input.clone();
            output.sort_unstable();
            (placed(&input), placed(&output))
        })
        .collect::<Vec<_>>();
    task(&format!("sort.{length}x{value}"), &reference, &example)
}

pub fn gnome(length: usize, value: usize) -> Task {
    let mut reference = format!("[Gnome.{}] Gnome.{},\n", POSITION[0], POSITION[1]);
    for index in 1..length {
        let (left, right) = (POSITION[index - 1], POSITION[index]);
        for first in 0..value {
            for second in 0..value {
                let (low, high, next) = if first <= second {
                    (first, second, POSITION[index + 1])
                } else {
                    (second, first, POSITION[index - 1])
                };
                reference.push_str(&format!(
                    "[Gnome.{right}, {left}.{first}, {right}.{second}] (Gnome.{next}, {left}.{low}, {right}.{high}),\n"
                ));
            }
        }
    }
    reference.push_str(&format!("[Gnome.{}],\n", POSITION[length]));
    let example = assignment(length, value)
        .into_iter()
        .map(|input| {
            let mut output = input.clone();
            output.sort_unstable();
            (
                format!("Gnome.{}, {}", POSITION[0], placed(&input)),
                placed(&output),
            )
        })
        .collect::<Vec<_>>();
    task(&format!("gnome.{length}x{value}"), &reference, &example)
}

pub fn reverse(length: usize, value: usize) -> Task {
    let mut reference = String::new();
    for (left, near) in POSITION.iter().take(length / 2).enumerate() {
        let far = POSITION[length - 1 - left];
        for first in 0..value {
            for second in 0..value {
                reference.push_str(&format!(
                    "[{near}.{first}.Flip, {far}.{second}.Flip] ({near}.{second}, {far}.{first}),\n"
                ));
            }
        }
    }
    let marked = |value: &[usize]| {
        value
            .iter()
            .enumerate()
            .map(|(index, value)| {
                let flip = if 2 * index + 1 == length { "" } else { ".Flip" };
                format!("{}.{value}{flip}", POSITION[index])
            })
            .collect::<Vec<_>>()
            .join(", ")
    };
    let example = assignment(length, value)
        .into_iter()
        .map(|input| {
            let output = input.iter().rev().copied().collect::<Vec<_>>();
            (marked(&input), placed(&output))
        })
        .collect::<Vec<_>>();
    task(&format!("reverse.{length}x{value}"), &reference, &example)
}

fn multiset(length: usize, value: usize) -> Vec<Vec<usize>> {
    assignment(length, value)
        .into_iter()
        .filter(|entry| entry.windows(2).all(|pair| pair[0] <= pair[1]))
        .collect()
}

fn reduction(name: &str, length: usize, value: usize, choose: fn(usize, usize) -> usize) -> Task {
    let mut reference = String::new();
    for high in 0..value {
        for low in 0..=high {
            reference.push_str(&format!(
                "[{name}.{low}, {name}.{high}] {name}.{},\n",
                choose(low, high)
            ));
        }
    }
    let example = multiset(length, value)
        .into_iter()
        .map(|input| {
            let result = input
                .iter()
                .copied()
                .reduce(choose)
                .expect("a reduction has operands");
            (
                input
                    .iter()
                    .map(|value| format!("{name}.{value}"))
                    .collect::<Vec<_>>()
                    .join(", "),
                format!("{name}.{result}"),
            )
        })
        .collect::<Vec<_>>();
    task(
        &format!("{}.{length}x{value}", name.to_lowercase()),
        &reference,
        &example,
    )
}

pub fn maximum(length: usize, value: usize) -> Task {
    reduction("Maximum", length, value, usize::max)
}

pub fn minimum(length: usize, value: usize) -> Task {
    reduction("Minimum", length, value, usize::min)
}

fn unary(tag: &str, count: usize) -> String {
    std::iter::once(tag.to_owned())
        .chain((0..count).map(|_| "Unit".to_owned()))
        .collect::<Vec<_>>()
        .join(".")
}

pub fn addition(limit: usize) -> Task {
    let mut example = Vec::new();
    for left in 0..=limit {
        for right in 0..=limit {
            let output = if left + right == 0 {
                "()".to_owned()
            } else {
                vec!["Unit"; left + right].join(".")
            };
            example.push((
                format!("{}, {}", unary("Left", left), unary("Right", right)),
                output,
            ));
        }
    }
    task(
        &format!("addition.{limit}"),
        "[Left, Right] Sum,\n[Sum] (),\n",
        &example,
    )
}

pub fn gather(length: usize, value: usize) -> Task {
    let mut reference = String::new();
    for high in 0..value {
        for low in 0..=high {
            reference.push_str(&format!("[Item.{low}, Item.{high}] Item.{low}.{high},\n"));
        }
    }
    let example = multiset(length, value)
        .into_iter()
        .map(|input| {
            (
                input
                    .iter()
                    .map(|value| format!("Item.{value}"))
                    .collect::<Vec<_>>()
                    .join(", "),
                std::iter::once("Item".to_owned())
                    .chain(input.iter().map(ToString::to_string))
                    .collect::<Vec<_>>()
                    .join("."),
            )
        })
        .collect::<Vec<_>>();
    task(&format!("gather.{length}x{value}"), &reference, &example)
}

pub fn copy(value: usize) -> Task {
    let reference = (0..value)
        .map(|value| format!("[Copy.{value}] ({value}, {value}),\n"))
        .collect::<String>();
    let example = (0..value)
        .map(|value| (format!("Copy.{value}"), format!("{value}, {value}")))
        .collect::<Vec<_>>();
    task(&format!("copy.{value}"), &reference, &example)
}

pub fn compare(value: usize) -> Task {
    let mut reference = String::new();
    let mut example = Vec::new();
    for left in 0..value {
        for right in 0..value {
            let verdict = match left.cmp(&right) {
                std::cmp::Ordering::Less => "Less",
                std::cmp::Ordering::Equal => "Equal",
                std::cmp::Ordering::Greater => "Greater",
            };
            reference.push_str(&format!("[Left.{left}, Right.{right}] {verdict},\n"));
            example.push((format!("Left.{left}, Right.{right}"), verdict.to_owned()));
        }
    }
    task(&format!("compare.{value}"), &reference, &example)
}

pub fn adder() -> Task {
    let example = [
        ("Add.0.0", "Sum.0.Carry.0"),
        ("Add.0.1", "Sum.1.Carry.0"),
        ("Add.1.1", "Sum.0.Carry.1"),
    ]
    .map(|(input, output)| (input.to_owned(), output.to_owned()));
    let reference = example
        .iter()
        .map(|(input, output)| format!("[{input}] {output},\n"))
        .collect::<String>();
    task("adder.half", &reference, &example)
}

pub fn curated() -> Vec<Task> {
    let mut result = boolean();
    result.extend([
        sort(2, 3),
        sort(3, 2),
        sort(3, 3),
        sort(4, 2),
        sort(5, 2),
        gnome(3, 3),
        gnome(4, 2),
        gnome(5, 2),
        holdout(gnome(4, 3), 27),
        holdout(gnome(6, 2), 24),
        holdout(gnome(5, 3), 27),
        holdout(gnome(7, 2), 24),
        reverse(2, 3),
        reverse(3, 3),
        reverse(4, 2),
        maximum(3, 3),
        maximum(4, 3),
        minimum(3, 3),
        addition(3),
        gather(3, 3),
        copy(3),
        compare(3),
        adder(),
    ]);
    result
}
