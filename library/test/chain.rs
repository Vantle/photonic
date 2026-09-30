use crate::catalog::source;
use crate::isolation::atom;
use photonic::laser::Laser;
use photonic::runtime::Limit;

// Whether every schedule of plain events that pushes item onto the empty chain and reads it back
// ends with exactly that item beside the empty chain.
fn kept(item: &str) -> bool {
    let program = crate::program(
        &format!("Push.{item}.Zero, [Drop.{item}] Forget, [Built] Read"),
        &[source("chain", "cell")],
    );
    let target = crate::target(&program, &format!("Yield.{item}.Zero"));
    let mut laser = Laser::reduced(&program);
    laser.run(
        1_000_000,
        Limit {
            configuration: 4096,
            record: 1_000_000,
            occurrence: 256,
            coherence: 64,
            scope: 64,
        },
    );
    assert!(laser.summary().closed, "{item}");
    let ending = laser.ending();
    let witness = laser.verdict(&target).witness;
    !ending.endless && ending.end.iter().copied().map(Some).eq([witness])
}

// A pattern takes any coherence that contains its atoms, so an item that is one of the cell's words
// can meet a pattern of the cell, and some schedules then lose it. Wrapped in a field keyed by a
// word of the caller's own, the same word is only an item.
#[test]
fn item() {
    let word = atom(source("chain", "cell"));
    let lost = word
        .iter()
        .filter(|word| !kept(word))
        .map(String::as_str)
        .collect::<Vec<_>>();
    assert_eq!(
        lost,
        [
            "Build", "Built", "Clean", "Discard", "Forget", "Free", "Handle", "Peek", "Push",
            "Read", "Request", "Restore", "Seal", "Shed",
        ]
    );
    for word in &word {
        assert!(kept(&format!("([Letter] {word})")), "{word}");
    }
    assert!(kept("Item"));
}
