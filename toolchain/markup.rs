use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Component, Path, PathBuf};

pub fn reference(text: &str) -> impl Iterator<Item = &str> {
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

pub fn document(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|value| value.to_str()),
        Some("md" | "html")
    )
}

pub fn resolve(base: &Path, target: &str) -> Option<PathBuf> {
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

pub fn check(
    page: &BTreeMap<PathBuf, String>,
    path: &Path,
    target: &str,
    exists: impl Fn(&Path) -> bool,
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
    if !exists(&destination) {
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

pub fn mention(text: &str) -> impl Iterator<Item = String> {
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
            && rest
                .starts_with(|value: char| value.is_ascii_alphanumeric() || "_:".contains(value))
            && !label.contains("...")
            && !label.ends_with('/')
            && !matches!(name, Some("all" | "*")))
        .then_some(label)
    })
}

pub fn package(label: &str) -> &str {
    label
        .split_once(':')
        .map_or(label, |(prefix, _)| prefix)
        .trim_start_matches('/')
}

pub fn dead(label: &str, packaged: bool, found: &BTreeSet<String>) -> Option<&'static str> {
    let target = label.contains(':');
    if !packaged {
        return Some(if target {
            "is not in a package"
        } else {
            "is not a package"
        });
    }
    (target && !found.contains(label)).then_some("is not a Bazel target")
}

#[cfg(test)]
#[path = "test/markup.rs"]
mod test;
