use frontend::source::Program;
use serde::Deserialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub struct Entry {
    pub name: String,
    pub group: String,
    pub program: Program,
}

#[derive(Deserialize)]
struct Case {
    program: String,
    source: String,
}

fn parse(text: &str) -> Option<Program> {
    frontend::lowering::parse(text).ok()
}

fn book(root: &Path, entry: &mut Vec<Entry>) {
    let text = std::fs::read_to_string(root.join("book/record.js")).expect("book/record.js");
    let marker = "globalThis.book.record = ";
    let start = text.find(marker).expect("record marker") + marker.len();
    let body = text[start..].trim_end().trim_end_matches(';');
    let body = body
        .lines()
        .filter(|line| *line != "__proto__: null,")
        .collect::<Vec<_>>()
        .join("\n");
    let value: serde_json::Value = serde_json::from_str(&body).expect("record json");
    let library = &value["library"];
    for section in ["example", "workbench"] {
        let Some(item) = value[section].as_object() else {
            continue;
        };
        for (name, item) in item {
            let mut program = Program::default();
            for dependency in item["library"].as_array().into_iter().flatten() {
                let dependency = dependency.as_str().unwrap();
                program.declare(
                    frontend::lowering::library(library[dependency].as_str().unwrap()).unwrap(),
                );
            }
            let Some(source) = parse(item["source"].as_str().unwrap()) else {
                continue;
            };
            program.append(source);
            entry.push(Entry {
                name: format!("book:{section}/{name}"),
                group: "book".into(),
                program,
            });
        }
    }
}

fn files(directory: &Path, suffix: &str, result: &mut Vec<PathBuf>) {
    let Ok(listing) = std::fs::read_dir(directory) else {
        return;
    };
    let mut listing = listing
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    listing.sort();
    for path in listing {
        if path.is_dir() {
            if !path
                .extension()
                .is_some_and(|extension| extension == "runfiles")
            {
                files(&path, suffix, result);
            }
        } else if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(suffix))
        {
            result.push(path);
        }
    }
}

fn label(bin: &Path, path: &Path, suffix: &str) -> (String, String) {
    let relative = path.strip_prefix(bin).unwrap().to_str().unwrap().to_owned();
    let (package, file) = relative.rsplit_once('/').unwrap();
    let target = file.trim_end_matches(suffix);
    let group = package.split('/').next().unwrap().to_owned();
    (format!("{package}:{target}"), group)
}

fn case(bin: &Path, entry: &mut Vec<Entry>) {
    let mut path = Vec::new();
    for package in ["program", "theorem", "library"] {
        files(&bin.join(package), ".case.case.json", &mut path);
    }
    for file in path {
        let Some(case) = std::fs::read_to_string(&file)
            .ok()
            .and_then(|text| serde_json::from_str::<Case>(&text).ok())
        else {
            continue;
        };
        let assembled = bin.join(case.program.trim_start_matches("_main/"));
        let Some(mut program) = std::fs::read_to_string(assembled)
            .ok()
            .and_then(|text| Program::read(&text).ok())
        else {
            continue;
        };
        let Some(source) = parse(&case.source) else {
            continue;
        };
        program.append(source);
        let (name, group) = label(bin, &file, ".case.case.json");
        entry.push(Entry {
            name,
            group,
            program,
        });
    }
}

fn binary(bin: &Path, entry: &mut Vec<Entry>) {
    let mut path = Vec::new();
    for package in ["program", "theorem", "library"] {
        files(&bin.join(package), ".program.json", &mut path);
    }
    for file in path {
        let Some(program) = std::fs::read_to_string(&file)
            .ok()
            .and_then(|text| Program::read(&text).ok())
        else {
            continue;
        };
        let (name, group) = label(bin, &file, ".program.json");
        entry.push(Entry {
            name,
            group,
            program,
        });
    }
}

fn escape(text: &str, byte: &[u8], mut cursor: usize, value: &mut Vec<u8>) -> usize {
    let next = byte.get(cursor + 1).copied();
    cursor += 2;
    match next {
        Some(b'n') => value.push(b'\n'),
        Some(b't') => value.push(b'\t'),
        Some(b'\\') => value.push(b'\\'),
        Some(b'"') => value.push(b'"'),
        Some(b'\'') => value.push(b'\''),
        Some(b'0') => value.push(0),
        Some(b'\n') => {
            while cursor < byte.len() && byte[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
        }
        Some(b'u') if byte.get(cursor) == Some(&b'{') => {
            if let Some(close) = text[cursor..].find('}') {
                let digit = &text[cursor + 1..cursor + close];
                if let Some(character) =
                    u32::from_str_radix(digit, 16).ok().and_then(char::from_u32)
                {
                    let mut buffer = [0; 4];
                    value.extend(character.encode_utf8(&mut buffer).as_bytes());
                }
                cursor += close + 1;
            }
        }
        Some(other) => {
            value.push(b'\\');
            value.push(other);
        }
        None => {}
    }
    cursor
}

fn literal(text: &str) -> Vec<String> {
    let byte = text.as_bytes();
    let mut result = Vec::new();
    let mut index = 0;
    while index < byte.len() {
        match byte[index] {
            b'/' if byte.get(index + 1) == Some(&b'/') => {
                while index < byte.len() && byte[index] != b'\n' {
                    index += 1;
                }
            }
            b'\'' => {
                if byte.get(index + 1) == Some(&b'\\') {
                    let mut cursor = index + 3;
                    while cursor < byte.len() && byte[cursor] != b'\'' {
                        cursor += 1;
                    }
                    index = cursor + 1;
                } else if byte.get(index + 2) == Some(&b'\'') {
                    index += 3;
                } else {
                    index += 1;
                }
            }
            b'r' if matches!(byte.get(index + 1), Some(b'"' | b'#'))
                && (index == 0 || !byte[index - 1].is_ascii_alphanumeric()) =>
            {
                let mut cursor = index + 1;
                let mut hash = 0;
                while byte.get(cursor) == Some(&b'#') {
                    hash += 1;
                    cursor += 1;
                }
                if byte.get(cursor) != Some(&b'"') {
                    index += 1;
                    continue;
                }
                let close = format!("\"{}", "#".repeat(hash));
                let start = cursor + 1;
                let Some(end) = text[start..].find(&close) else {
                    break;
                };
                result.push(text[start..start + end].to_owned());
                index = start + end + close.len();
            }
            b'"' => {
                let mut value = Vec::new();
                let mut cursor = index + 1;
                let mut closed = false;
                while cursor < byte.len() {
                    match byte[cursor] {
                        b'"' => {
                            closed = true;
                            cursor += 1;
                            break;
                        }
                        b'\\' => cursor = escape(text, byte, cursor, &mut value),
                        other => {
                            value.push(other);
                            cursor += 1;
                        }
                    }
                }
                if !closed {
                    break;
                }
                result.push(String::from_utf8_lossy(&value).into_owned());
                index = cursor;
            }
            _ => index += 1,
        }
    }
    result
}

fn language(root: &Path, entry: &mut Vec<Entry>) {
    let mut path = Vec::new();
    files(&root.join("language/test"), ".rs", &mut path);
    for file in path {
        let text = std::fs::read_to_string(&file).unwrap();
        let stem = file.file_stem().unwrap().to_str().unwrap().to_owned();
        for (index, value) in literal(&text).into_iter().enumerate() {
            let Some(program) = parse(&value) else {
                continue;
            };
            if program.rule.is_empty() || (program.initial.is_empty() && program.scope.is_empty()) {
                continue;
            }
            entry.push(Entry {
                name: format!("language/test/{stem}:{index}"),
                group: "language".into(),
                program,
            });
        }
    }
}

#[derive(Deserialize)]
struct Reference {
    name: String,
    program: Program,
}

fn reference(root: &Path, entry: &mut Vec<Entry>) {
    let text = std::fs::read_to_string(root.join("language/test/reference.json")).unwrap();
    let case: Vec<Reference> = serde_json::from_str(&text).unwrap();
    for value in case {
        entry.push(Entry {
            name: format!("language/reference:{}", value.name),
            group: "language".into(),
            program: value.program,
        });
    }
}

fn synthetic(entry: &mut Vec<Entry>) {
    for count in [2, 4, 6, 8] {
        entry.push(Entry {
            name: format!("synthetic:dial.{count}"),
            group: "synthetic".into(),
            program: parse(&photonic::family::dial(count)).unwrap(),
        });
    }
    for count in [2, 3, 4, 5] {
        entry.push(Entry {
            name: format!("synthetic:diner.{count}"),
            group: "synthetic".into(),
            program: parse(&photonic::family::diner(count)).unwrap(),
        });
    }
}

pub fn gather(root: &Path, bin: &Path) -> Vec<Entry> {
    let mut entry = Vec::new();
    book(root, &mut entry);
    case(bin, &mut entry);
    binary(bin, &mut entry);
    language(root, &mut entry);
    reference(root, &mut entry);
    synthetic(&mut entry);
    let mut seen = HashSet::new();
    entry.retain(|value| seen.insert(value.program.canonical()));
    entry
}
