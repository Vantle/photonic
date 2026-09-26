mod markup;

use relay::Failure;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

fn link() -> Result<ExitCode, Failure> {
    let workspace = PathBuf::from(
        std::env::var_os("BUILD_WORKSPACE_DIRECTORY")
            .ok_or(Failure::Usage("run the link check with bazel run //:link"))?,
    );
    let page = crawl(
        &workspace,
        std::env::args().skip(1).map(PathBuf::from).collect(),
    )?;
    let exists = |destination: &Path| workspace.join(destination).exists();
    let mut problem = Vec::new();
    let mut label = BTreeMap::new();
    for (path, text) in &page {
        for target in markup::reference(text) {
            if let Some(message) = markup::check(&page, path, target, exists) {
                problem.push(format!("{}: {message}", path.display()));
            }
        }
        for value in markup::mention(text) {
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

fn crawl(workspace: &Path, root: Vec<PathBuf>) -> Result<BTreeMap<PathBuf, String>, Failure> {
    let mut page = BTreeMap::new();
    let mut pending = root;
    while let Some(path) = pending.pop() {
        if page.contains_key(&path) {
            continue;
        }
        let file = workspace.join(&path);
        let text = std::fs::read_to_string(&file).map_err(Failure::access(&file))?;
        pending.extend(
            markup::reference(&text)
                .filter_map(|target| {
                    markup::resolve(&path, target.split('#').next().unwrap_or_default())
                })
                .filter(|target| markup::document(target) && workspace.join(target).is_file()),
        );
        page.insert(path, text);
    }
    Ok(page)
}

fn query(workspace: &Path, label: &BTreeMap<String, PathBuf>) -> Result<Vec<String>, Failure> {
    let packaged = |value: &str| {
        workspace
            .join(markup::package(value))
            .join("BUILD.bazel")
            .is_file()
    };
    let target = label
        .keys()
        .map(String::as_str)
        .filter(|value| value.contains(':') && packaged(value))
        .collect::<Vec<_>>();
    let found = find(workspace, &target)?;
    Ok(label
        .iter()
        .filter_map(|(value, path)| {
            let message = markup::dead(value, packaged(value), &found)?;
            Some(format!("{}: {value} {message}", path.display()))
        })
        .collect())
}

fn find(workspace: &Path, target: &[&str]) -> Result<BTreeSet<String>, Failure> {
    if target.is_empty() {
        return Ok(BTreeSet::new());
    }
    let bazel = PathBuf::from(std::env::var_os("BAZEL_REAL").unwrap_or_else(|| "bazel".into()));
    let output = Command::new(&bazel)
        .current_dir(workspace)
        .args([
            "query",
            &format!("set({})", target.join(" ")),
            "--keep_going",
            "--output=label",
        ])
        .stderr(Stdio::null())
        .output()
        .map_err(Failure::access(&bazel))?;
    let text = String::from_utf8(output.stdout).map_err(|error| Failure::Tool {
        path: bazel,
        message: error.to_string(),
    })?;
    Ok(text.lines().map(str::to_owned).collect())
}

fn main() -> ExitCode {
    link().unwrap_or_else(Failure::report)
}
