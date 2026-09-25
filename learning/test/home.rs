use crate::home::Home;
use crate::objective::Setting;
use crate::pool::{grow, initial};

fn directory(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("learning-{name}-{}", std::process::id()))
}

#[test]
fn persistence() {
    let path = directory("home");
    let home = Home::open(path.clone()).unwrap();
    assert_eq!(home.load::<Vec<u32>>("value.json").unwrap(), None);
    home.save("value.json", &vec![1u32, 2, 3]).unwrap();
    assert_eq!(
        home.load::<Vec<u32>>("value.json").unwrap(),
        Some(vec![1, 2, 3])
    );
    home.append("line.jsonl", &1u32).unwrap();
    home.append("line.jsonl", &2u32).unwrap();
    assert_eq!(
        std::fs::read_to_string(home.file("line.jsonl")).unwrap(),
        "1\n2\n"
    );
    std::fs::write(home.file("broken.json"), "{").unwrap();
    assert!(home.load::<Vec<u32>>("broken.json").is_err());
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn growth() {
    let setting = Setting::default();
    let pool = initial(2, 5, &setting).unwrap();
    let before = pool.len();
    let grown = grow(pool, 3, 5, &setting).unwrap();
    assert_eq!(grown.len(), before + 3);
    let mut name = grown
        .iter()
        .map(|task| task.name.clone())
        .collect::<Vec<_>>();
    name.sort();
    name.dedup();
    assert_eq!(name.len(), grown.len());
    assert!(grown.iter().any(|task| task.name == "synthetic.4"));
}
