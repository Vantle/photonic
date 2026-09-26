use crate::runtime::Limit;
fn enumerate(candidate: &[Vec<usize>], used: &mut Vec<usize>) -> bool {
    if used.len() == candidate.len() {
        return true;
    }
    for &world in &candidate[used.len()] {
        if used.contains(&world) {
            continue;
        }
        used.push(world);
        if enumerate(candidate, used) {
            return true;
        }
        used.pop();
    }
    false
}

#[test]
fn exhaustive() {
    for position in 0..=4 {
        for count in 0..=4 {
            for encoding in 0..1usize << (position * count) {
                let candidate = (0..position)
                    .map(|position| {
                        (0..count)
                            .filter(|world| encoding & (1 << (position * count + world)) != 0)
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>();
                assert_eq!(
                    crate::assignment::feasible(&candidate),
                    enumerate(&candidate, &mut Vec::new()),
                    "{candidate:?}"
                );
            }
        }
    }
}

#[test]
fn shortage() {
    for separation in [".", ","] {
        let initial = vec!["A"; 18].join(separation);
        let input = vec!["A"; 19].join(separation);
        let source = format!("{initial}, [{input}] B");
        let mut runtime =
            crate::runtime::Runtime::new(&frontend::lowering::parse(&source).unwrap());
        runtime.run(100, Limit::default());
        assert!(runtime.closed());
        let snapshot = runtime.snapshot();
        assert_eq!(snapshot.state.len(), 1);
        assert!(snapshot.event.is_empty());
    }
}
