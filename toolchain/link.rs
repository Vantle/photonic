use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::{Error, ErrorKind};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

fn main() -> Result<ExitCode, Error> {
    let workspace = PathBuf::from(std::env::var_os("BUILD_WORKSPACE_DIRECTORY").ok_or_else(
        || {
            Error::new(
                ErrorKind::NotFound,
                "run the link check with bazel run //:link",
            )
        },
    )?);
    let page = crawl(
        &workspace,
        std::env::args().skip(1).map(PathBuf::from).collect(),
    )?;
    let mut problem = Vec::new();
    let mut label = BTreeMap::new();
    for (path, text) in &page {
        for target in reference(text) {
            if let Some(message) = check(&workspace, &page, path, target) {
                problem.push(format!("{}: {message}", path.display()));
            }
        }
        for value in mention(text) {
            label.entry(value).or_insert_with(|| path.clone());
        }
    }
    problem.extend(query(&workspace, &label)?);
    for line in &problem {
        eprintln!("{line}");
    }
    if !problem.is_empty() {
        return Ok(ExitCode::FAILURE);
    }
    println!(
        "{} pages and {} labels have no dead references.",
        page.len(),
        label.len()
    );
    Ok(ExitCode::SUCCESS)
}

fn crawl(workspace: &Path, root: Vec<PathBuf>) -> Result<BTreeMap<PathBuf, String>, Error> {
    let mut page = BTreeMap::new();
    let mut pending = root;
    while let Some(path) = pending.pop() {
        if page.contains_key(&path) {
            continue;
        }
        let text = std::fs::read_to_string(workspace.join(&path))?;
        pending.extend(
            reference(&text)
                .filter_map(|target| resolve(&path, target.split('#').next().unwrap_or_default()))
                .filter(|target| document(target) && workspace.join(target).is_file()),
        );
        page.insert(path, text);
    }
    Ok(page)
}

fn reference(text: &str) -> impl Iterator<Item = &str> {
    ["](", "href=\"", "src=\"", "srcset=\""]
        .into_iter()
        .flat_map(move |opening| {
            let closing = if opening == "](" { ')' } else { '"' };
            text.match_indices(opening)
                .filter_map(move |(start, _)| {
                    let rest = &text[start + opening.len()..];
                    rest.find(closing).map(|end| &rest[..end])
                })
                .flat_map(move |value| candidate(opening, value))
        })
        .filter(|target| !target.is_empty() && !external(target))
}

fn candidate<'text>(opening: &str, value: &'text str) -> Vec<&'text str> {
    if opening != "srcset=\"" {
        return vec![value];
    }
    value
        .split(',')
        .filter_map(|entry| entry.split_whitespace().next())
        .collect()
}

fn external(target: &str) -> bool {
    target.starts_with("//")
        || target.split_once(':').is_some_and(|(scheme, _)| {
            !scheme.is_empty() && scheme.chars().all(|value| value.is_ascii_alphanumeric())
        })
}

fn document(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|value| value.to_str()),
        Some("md" | "html")
    )
}

fn resolve(base: &Path, target: &str) -> Option<PathBuf> {
    let mut path = base.parent().unwrap_or(Path::new("")).to_path_buf();
    for component in Path::new(&target.replace("&amp;", "&")).components() {
        match component {
            Component::Normal(part) => path.push(part),
            Component::CurDir => {}
            Component::ParentDir => {
                if !path.pop() {
                    return None;
                }
            }
            _ => return None,
        }
    }
    Some(path)
}

fn check(
    workspace: &Path,
    page: &BTreeMap<PathBuf, String>,
    path: &Path,
    target: &str,
) -> Option<String> {
    let (file, fragment) = target.split_once('#').unwrap_or((target, ""));
    let Some(destination) = resolve(path, file) else {
        return Some(format!("{target} leaves the workspace"));
    };
    let destination = if file.is_empty() {
        path.to_path_buf()
    } else {
        destination
    };
    if !workspace.join(&destination).exists() {
        return Some(format!("{target} does not exist"));
    }
    if fragment.is_empty() || !document(&destination) {
        return None;
    }
    let text = page.get(&destination)?;
    if anchor(&destination, text).contains(fragment) {
        return None;
    }
    Some(format!("{target} names a missing anchor"))
}

fn anchor(path: &Path, text: &str) -> BTreeSet<String> {
    let mut anchor = text
        .match_indices("id=\"")
        .filter_map(|(start, _)| {
            let rest = &text[start + 4..];
            rest.find('"').map(|end| rest[..end].to_owned())
        })
        .collect::<BTreeSet<_>>();
    if path.extension().and_then(|value| value.to_str()) != Some("md") {
        return anchor;
    }
    let mut fenced = false;
    let mut count = HashMap::new();
    for line in text.lines() {
        if line.starts_with("```") {
            fenced = !fenced;
        }
        let Some(heading) = line.strip_prefix('#').filter(|_| !fenced) else {
            continue;
        };
        let slug = heading
            .trim_start_matches('#')
            .trim()
            .to_lowercase()
            .chars()
            .filter(|value| value.is_alphanumeric() || matches!(value, ' ' | '-' | '_'))
            .map(|value| if value == ' ' { '-' } else { value })
            .collect::<String>();
        let repeat = count.entry(slug.clone()).or_insert(0);
        anchor.insert(if *repeat == 0 {
            slug
        } else {
            format!("{slug}-{repeat}")
        });
        *repeat += 1;
    }
    anchor
}

fn mention(text: &str) -> impl Iterator<Item = String> {
    text.match_indices("//").filter_map(move |(start, _)| {
        let before = text[..start].chars().next_back();
        if before.is_some_and(|value| value.is_ascii_alphanumeric() || "_@/:.".contains(value)) {
            return None;
        }
        let rest = &text[start + 2..];
        let end = rest
            .find(|value: char| !(value.is_ascii_alphanumeric() || "_-/.:+".contains(value)))
            .unwrap_or(rest.len());
        let label = format!("//{}", rest[..end].trim_end_matches(['.', ':']));
        let name = label.split_once(':').map(|(_, name)| name);
        (label.len() > 2
            && rest.starts_with(|value: char| value.is_ascii_alphanumeric() || value == '_')
            && !label.contains("...")
            && !label.ends_with('/')
            && !matches!(name, Some("all" | "*")))
        .then_some(label)
    })
}

fn query(workspace: &Path, label: &BTreeMap<String, PathBuf>) -> Result<Vec<String>, Error> {
    let mut problem = Vec::new();
    let mut target = Vec::new();
    for (value, path) in label {
        let Some((package, _)) = value.split_once(':') else {
            if !workspace
                .join(value.trim_start_matches('/'))
                .join("BUILD.bazel")
                .is_file()
            {
                problem.push(format!("{}: {value} is not a package", path.display()));
            }
            continue;
        };
        if !workspace
            .join(package.trim_start_matches('/'))
            .join("BUILD.bazel")
            .is_file()
        {
            problem.push(format!("{}: {value} is not in a package", path.display()));
            continue;
        }
        target.push(value.as_str());
    }
    if target.is_empty() {
        return Ok(problem);
    }
    let output = Command::new(std::env::var_os("BAZEL_REAL").unwrap_or_else(|| "bazel".into()))
        .current_dir(workspace)
        .args([
            "query",
            &format!("set({})", target.join(" ")),
            "--keep_going",
            "--output=label",
        ])
        .stderr(Stdio::null())
        .output()?;
    let found = String::from_utf8(output.stdout)
        .map_err(Error::other)?
        .lines()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    problem.extend(
        target
            .into_iter()
            .filter(|value| !found.contains(*value))
            .map(|value| format!("{}: {value} is not a Bazel target", label[value].display())),
    );
    Ok(problem)
}
