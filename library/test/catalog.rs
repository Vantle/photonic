use std::collections::BTreeSet;
use std::path::Path;
use std::sync::LazyLock;

const PACKAGE: [&str; 14] = [
    "binary",
    "boolean",
    "carry",
    "chain",
    "collection",
    "expression",
    "field",
    "function",
    "integer",
    "natural",
    "selection",
    "stream",
    "ternary",
    "vector",
];

pub struct Entry {
    pub package: String,
    pub name: String,
    pub source: String,
}

pub static LIBRARY: LazyLock<Vec<Entry>> = LazyLock::new(|| {
    let runfile = runfiles::Runfiles::create().unwrap();
    let mut entry = std::env::var("PHOTONIC_LIBRARY")
        .unwrap()
        .split_whitespace()
        .map(|path| {
            let file = Path::new(path);
            let text =
                |value: Option<&std::ffi::OsStr>| value.unwrap().to_str().unwrap().to_owned();
            Entry {
                package: text(file.parent().and_then(Path::file_name)),
                name: text(file.file_stem()),
                source: std::fs::read_to_string(runfile.rlocation_from(path, "").unwrap()).unwrap(),
            }
        })
        .collect::<Vec<_>>();
    entry.sort_by(|left, right| (&left.package, &left.name).cmp(&(&right.package, &right.name)));
    let package = entry
        .iter()
        .map(|entry| entry.package.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        package,
        BTreeSet::from(PACKAGE),
        "PHOTONIC_LIBRARY loads every library package"
    );
    entry
});

pub fn source(package: &str, name: &str) -> &'static str {
    &LIBRARY
        .iter()
        .find(|entry| entry.package == package && entry.name == name)
        .unwrap()
        .source
}
