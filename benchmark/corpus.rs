use frontend::source::Program;
use serde::Deserialize;
use std::collections::HashSet;
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum Failure {
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    Parse {
        origin: String,
        message: String,
    },
    Empty {
        bin: PathBuf,
    },
}

impl fmt::Display for Failure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(formatter, "could not read {}: {source}", path.display())
            }
            Self::Parse { origin, message } => {
                write!(formatter, "could not parse {origin}: {message}")
            }
            Self::Empty { bin } => write!(
                formatter,
                "{} holds no assembled programs or cases; build the repository with -c opt first",
                bin.display()
            ),
        }
    }
}

impl std::error::Error for Failure {}

fn read(path: &Path) -> Result<String, Failure> {
    std::fs::read_to_string(path).map_err(|source| Failure::Read {
        path: path.to_path_buf(),
        source,
    })
}

fn malformed(origin: &Path, message: impl ToString) -> Failure {
    Failure::Parse {
        origin: origin.display().to_string(),
        message: message.to_string(),
    }
}

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

fn book(root: &Path, entry: &mut Vec<Entry>) -> Result<(), Failure> {
    let path = root.join("book/record.js");
    let text = read(&path)?;
    let marker = "globalThis.book.record = ";
    let start = text
        .find(marker)
        .ok_or_else(|| malformed(&path, "no record marker"))?
        + marker.len();
    let body = text[start..]
        .trim_end()
        .trim_end_matches(';')
        .lines()
        .filter(|line| *line != "__proto__: null,")
        .collect::<Vec<_>>()
        .join("\n");
    let value: serde_json::Value =
        serde_json::from_str(&body).map_err(|error| malformed(&path, error))?;
    let library = &value["library"];
    for section in ["example", "workbench"] {
        let Some(item) = value[section].as_object() else {
            continue;
        };
        for (name, item) in item {
            let mut program = Program::default();
            for dependency in item["library"].as_array().into_iter().flatten() {
                let dependency = dependency
                    .as_str()
                    .ok_or_else(|| malformed(&path, format!("{name} names a library badly")))?;
                let source = library[dependency]
                    .as_str()
                    .ok_or_else(|| malformed(&path, format!("no library {dependency}")))
                    .and_then(|text| {
                        frontend::lowering::parse(text).map_err(|error| malformed(&path, error))
                    })?;
                program
                    .declare(source, dependency)
                    .map_err(|error| malformed(&path, error))?;
            }
            let source = item["source"]
                .as_str()
                .ok_or_else(|| malformed(&path, format!("{name} has no source")))
                .and_then(|text| {
                    frontend::lowering::parse(text).map_err(|error| malformed(&path, error))
                })?;
            program.append(source);
            entry.push(Entry {
                name: format!("book:{section}/{name}"),
                group: "book".into(),
                program,
            });
        }
    }
    Ok(())
}

// A package with nothing built has no directory under bin, which the check for an empty bin
// reports as a whole.
fn walk(directory: &Path, suffix: &str, result: &mut Vec<PathBuf>) -> Result<(), Failure> {
    if !directory.exists() {
        return Ok(());
    }
    let listing = std::fs::read_dir(directory).map_err(|source| Failure::Read {
        path: directory.to_path_buf(),
        source,
    })?;
    let mut listing = listing
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| Failure::Read {
            path: directory.to_path_buf(),
            source,
        })?;
    listing.sort();
    for path in listing {
        if path.is_dir() {
            if !path
                .extension()
                .is_some_and(|extension| extension == "runfiles")
            {
                walk(&path, suffix, result)?;
            }
        } else if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(suffix))
        {
            result.push(path);
        }
    }
    Ok(())
}

fn label(bin: &Path, path: &Path, suffix: &str) -> Result<(String, String), Failure> {
    let relative = path
        .strip_prefix(bin)
        .ok()
        .and_then(Path::to_str)
        .ok_or_else(|| malformed(path, "names no package under bin"))?;
    let (package, file) = relative
        .rsplit_once('/')
        .ok_or_else(|| malformed(path, "names no package under bin"))?;
    let target = file.trim_end_matches(suffix);
    let group = package.split('/').next().unwrap_or(package).to_owned();
    Ok((format!("{package}:{target}"), group))
}

fn case(bin: &Path, entry: &mut Vec<Entry>) -> Result<(), Failure> {
    let mut path = Vec::new();
    for package in ["program", "theorem", "library"] {
        walk(&bin.join(package), ".case.case.json", &mut path)?;
    }
    for file in path {
        let case =
            serde_json::from_str::<Case>(&read(&file)?).map_err(|error| malformed(&file, error))?;
        let assembled = bin.join(case.program.trim_start_matches("_main/"));
        let mut program =
            Program::read(&read(&assembled)?).map_err(|error| malformed(&assembled, error))?;
        let source =
            frontend::lowering::parse(&case.source).map_err(|error| malformed(&file, error))?;
        program.append(source);
        let (name, group) = label(bin, &file, ".case.case.json")?;
        entry.push(Entry {
            name,
            group,
            program,
        });
    }
    Ok(())
}

fn binary(bin: &Path, entry: &mut Vec<Entry>) -> Result<(), Failure> {
    let mut path = Vec::new();
    for package in ["program", "theorem", "library"] {
        walk(&bin.join(package), ".program.json", &mut path)?;
    }
    for file in path {
        let program = Program::read(&read(&file)?).map_err(|error| malformed(&file, error))?;
        let (name, group) = label(bin, &file, ".program.json")?;
        entry.push(Entry {
            name,
            group,
            program,
        });
    }
    Ok(())
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

fn language(root: &Path, entry: &mut Vec<Entry>) -> Result<(), Failure> {
    let mut path = Vec::new();
    walk(&root.join("language/test"), ".rs", &mut path)?;
    for file in path {
        let text = read(&file)?;
        let stem = file
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| malformed(&file, "has no name"))?
            .to_owned();
        for (index, value) in literal(&text).into_iter().enumerate() {
            // A test's string literals are programs only some of the time.
            let Ok(program) = frontend::lowering::parse(&value) else {
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
    Ok(())
}

#[derive(Deserialize)]
struct Reference {
    name: String,
    program: Program,
}

fn reference(root: &Path, entry: &mut Vec<Entry>) -> Result<(), Failure> {
    let path = root.join("language/test/reference.json");
    let case: Vec<Reference> =
        serde_json::from_str(&read(&path)?).map_err(|error| malformed(&path, error))?;
    for value in case {
        entry.push(Entry {
            name: format!("language/reference:{}", value.name),
            group: "language".into(),
            program: value.program,
        });
    }
    Ok(())
}

fn synthetic(entry: &mut Vec<Entry>) -> Result<(), Failure> {
    let family = |name: &str, text: String| {
        frontend::lowering::parse(&text).map_err(|error| Failure::Parse {
            origin: name.to_owned(),
            message: error.to_string(),
        })
    };
    for count in [2, 4, 6, 8] {
        let name = format!("synthetic:dial.{count}");
        entry.push(Entry {
            program: family(&name, photonic::family::dial(count))?,
            name,
            group: "synthetic".into(),
        });
    }
    for count in [2, 3, 4, 5] {
        let name = format!("synthetic:diner.{count}");
        entry.push(Entry {
            program: family(&name, photonic::family::diner(count))?,
            name,
            group: "synthetic".into(),
        });
    }
    Ok(())
}

fn assembled(bin: &Path) -> Result<Vec<Entry>, Failure> {
    let mut entry = Vec::new();
    case(bin, &mut entry)?;
    binary(bin, &mut entry)?;
    if entry.is_empty() {
        return Err(Failure::Empty {
            bin: bin.to_path_buf(),
        });
    }
    Ok(entry)
}

pub fn gather(root: &Path, bin: &Path) -> Result<Vec<Entry>, Failure> {
    let mut entry = Vec::new();
    book(root, &mut entry)?;
    entry.extend(assembled(bin)?);
    language(root, &mut entry)?;
    reference(root, &mut entry)?;
    synthetic(&mut entry)?;
    let mut seen = HashSet::new();
    entry.retain(|value| seen.insert(value.program.canonical()));
    Ok(entry)
}

#[cfg(test)]
#[path = "test/corpus.rs"]
mod test;
