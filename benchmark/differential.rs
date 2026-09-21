use serde_json::Value;

fn observation(value: impl serde::Serialize) -> Value {
    let mut value = serde_json::to_value(value).unwrap();
    let object = value.as_object_mut().unwrap();
    object.remove("record");
    object.remove("peak");
    value
}

fn exhaustive(source: &str) -> usize {
    bounded(
        source,
        photonic::runtime::Limit::default(),
        &[0, 1, 2, 7, 31, 127, 511, 2048],
        false,
    )
}

fn bounded(
    source: &str,
    limit: photonic::runtime::Limit,
    budget: &[usize],
    pressure: bool,
) -> usize {
    let mut current = photonic::runtime::Runtime::new(photonic::lowering::parse(source).unwrap());
    let mut previous =
        reference::runtime::Runtime::new(reference::lowering::parse(source).unwrap());
    let bound = reference::runtime::Limit {
        state: limit.state,
        record: limit.record,
        world: limit.world,
        cell: limit.cell,
        frame: limit.frame,
    };
    for &budget in budget {
        if pressure && budget == 2048 {
            current.run(1, Some(photonic::runtime::Limit { record: 1, ..limit }));
            previous.run(1, Some(reference::runtime::Limit { record: 1, ..bound }));
            assert_eq!(
                observation(current.snapshot()),
                observation(previous.snapshot()),
                "suspension: {source}"
            );
        }
        current.run(budget, Some(limit));
        previous.run(budget, Some(bound));
        assert_eq!(
            observation(current.snapshot()),
            observation(previous.snapshot()),
            "{source}, budget {budget}"
        );
    }
    budget.len() + usize::from(pressure)
}

fn direct(source: &str, target: &str) -> usize {
    let mut current = photonic::path::Search::new(
        photonic::lowering::parse(source).unwrap(),
        photonic::lowering::parse(target).unwrap(),
    )
    .unwrap();
    let mut previous = reference::path::Search::new(
        reference::lowering::parse(source).unwrap(),
        reference::lowering::parse(target).unwrap(),
    )
    .unwrap();
    let limit = photonic::runtime::Limit {
        state: 256,
        record: 2000000,
        world: 32,
        cell: 2048,
        frame: 128,
    };
    let bound = reference::runtime::Limit {
        state: 256,
        record: 2000000,
        world: 32,
        cell: 2048,
        frame: 128,
    };
    for budget in [0, 1, 2, 7, 31, 127, 511, 2048, 16384] {
        current.run(budget, limit);
        previous.run(budget, bound);
        assert_eq!(
            observation(current.report()),
            observation(previous.report()),
            "{source}, budget {budget}"
        );
    }
    9
}

fn wide() -> usize {
    let common = (0..512)
        .map(|position| format!("Item{position}"))
        .collect::<Vec<_>>()
        .join(",");
    let source = format!("{common},X,Y,Z [{common},X] Done [{common},Y] Done [{common},Z] Done");
    let limit = photonic::runtime::Limit {
        state: 8,
        record: 2000000,
        world: 1024,
        cell: 2048,
        frame: 128,
    };
    let budget = [0, 1, 127, 511, 2048, 8192, 16384];
    bounded(&source, limit, &budget, false) + bounded(&source, limit, &budget, true)
}

fn composition() -> usize {
    let input = ["A"; 8].join(".");
    let intermediate = ["B"; 32].join(".");
    let output = ["C"; 32].join(".");
    let other = ["D"; 32].join(".");
    let source = format!(
        "{input} [{input}] {intermediate} [{intermediate}] {output} [{intermediate}] {other}"
    );
    let limit = photonic::runtime::Limit {
        cell: 128,
        ..Default::default()
    };
    let budget = [0, 1, 127, 511, 2048, 8192, 16384];
    bounded(&source, limit, &budget, false) + bounded(&source, limit, &budget, true)
}

fn run() -> usize {
    let mut count = wide() + composition();
    for source in ["A.B.C,A,B.C [A,B,C] Done", "A,C [A] B [C] D"] {
        count += bounded(
            source,
            photonic::runtime::Limit::default(),
            &[1; 128],
            false,
        );
    }

    for depth in [1, 2, 3, 8, 16, 32] {
        let mut rule = "Done".to_owned();
        let mut target = vec![rule.clone()];
        for position in (0..depth).rev() {
            rule = format!("[Step{position}] {rule}");
            if position > 0 {
                target.push(format!("({rule})"));
            }
        }
        let initial = (0..depth)
            .map(|position| format!("Step{position}"))
            .collect::<Vec<_>>()
            .join(".");
        let source = format!("{initial} {rule}");
        count += direct(&source, &target.join("."));
        if depth <= 3 {
            count += exhaustive(&source);
        }
        let mut value = "Done".to_owned();
        let mut other = "Wrong".to_owned();
        for _ in 0..depth {
            value = format!("[Absent] ({value})");
            other = format!("[Absent] ({other})");
        }
        let source = format!("({value}) [({other})] Forbidden [({value})] Accepted");
        count += direct(&source, "Accepted");
        if depth <= 8 {
            count += exhaustive(&source);
        }
    }
    for width in 1..=6 {
        for multiplicity in 1..=3 {
            let particle = vec!["A"; multiplicity].join(".");
            let initial = vec![particle.clone(); width].join(",");
            let source = format!("{initial} [{particle}] B [B] C [C] D");
            count += direct(&source, "Missing");
            count += exhaustive(&source);
        }
    }
    for source in [
        "Seed [Seed] (Again [Again] Again)",
        "Seed [Seed] (First [First] Second [Second] First)",
        "Again [Again] Again.([Absent] Done)",
        "Enter.Make.Call [Enter] ([A] Local [Make] [Call] (A [Local] Done)) [A] Global",
        "A,B [A,B] C,D [C,D] E",
        "A [A] B,C [B] D [C] E",
        "A [A] B [B] A",
    ] {
        count += direct(source, "Missing");
        count += exhaustive(source);
    }
    count
}

#[cfg(not(test))]
fn main() {
    println!(
        "{} budgeted observations excluding cache footprint agree with the pinned runtime",
        run()
    );
}

#[test]
fn regression() {
    assert!(run() > 500);
}
