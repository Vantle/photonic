use super::{anchor, check, dead, mention, package, reference, resolve};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[test]
fn root() {
    let text = "Run `bazel run //:install`, then `bazel run -c opt //:format -- --check`, \
        //:analyze.rust and //toolchain/browser:serve. The //library package is one too, \
        but //:all, //... and a bare //: are not, and neither are https://vantle.org//:page \
        or @crate//:serde.";
    assert_eq!(
        mention(text).collect::<Vec<_>>(),
        [
            "//:install",
            "//:format",
            "//:analyze.rust",
            "//toolchain/browser:serve",
            "//library",
        ]
    );
    assert_eq!(mention("//: and //:.").count(), 0);
}

#[test]
fn srcset() {
    let text = r#"<picture>
<source media="(prefers-color-scheme: dark)" srcset="book/logo/dark.svg 1x, book/logo/wide.svg 2x">
<img src="book/logo/light.svg" alt="Photonic">
</picture>
<a href="https://photonic.vantle.org">Book</a> <a href="mailto:someone@example.com">Mail</a>
<a href="//example.com/script.js">Script</a>
Read [the build](document/build.md#configuration) and [this section](#hosted-capacity)."#;
    assert_eq!(
        reference(text).collect::<Vec<_>>(),
        [
            "document/build.md#configuration",
            "#hosted-capacity",
            "book/logo/light.svg",
            "book/logo/dark.svg",
            "book/logo/wide.svg",
        ]
    );
}

#[test]
fn resolution() {
    let build = Path::new("document/build.md");
    assert_eq!(
        resolve(build, "../README.md"),
        Some(PathBuf::from("README.md"))
    );
    assert_eq!(
        resolve(build, "./automation.md"),
        Some(PathBuf::from("document/automation.md"))
    );
    assert_eq!(
        resolve(Path::new("index.html"), "a&amp;b.html"),
        Some(PathBuf::from("a&b.html"))
    );
    assert_eq!(resolve(Path::new("README.md"), "../outside.md"), None);
    assert_eq!(resolve(Path::new("README.md"), "/etc/hosts"), None);
}

#[test]
fn fragment() {
    let page = BTreeMap::from([
        (PathBuf::from("README.md"), String::new()),
        (
            PathBuf::from("document/build.md"),
            "# Build organization\n\n## Configuration\n".to_owned(),
        ),
        (
            PathBuf::from("index.html"),
            "<section id=\"value\"></section>".to_owned(),
        ),
    ]);
    let exists = |path: &Path| page.contains_key(path) || path == Path::new("book/icon.svg");
    let readme = Path::new("README.md");
    let problem = |path: &Path, target: &str| check(&page, path, target, exists);
    assert_eq!(problem(readme, "document/build.md#configuration"), None);
    assert_eq!(problem(readme, "index.html#value"), None);
    assert_eq!(
        problem(Path::new("document/build.md"), "#configuration"),
        None
    );
    assert_eq!(problem(readme, "book/icon.svg#unchecked"), None);
    assert_eq!(
        problem(readme, "document/build.md#release"),
        Some("document/build.md#release names a missing anchor".to_owned())
    );
    assert_eq!(
        problem(readme, "index.html#absent"),
        Some("index.html#absent names a missing anchor".to_owned())
    );
    assert_eq!(
        problem(readme, "document/absent.md"),
        Some("document/absent.md does not exist".to_owned())
    );
    assert_eq!(
        problem(readme, "../outside.md"),
        Some("../outside.md leaves the workspace".to_owned())
    );
}

#[test]
fn slug() {
    let text = "# Build organization\n\
        ## Build · Linux x86-64\n\
        ```sh\n\
        # not a heading\n\
        ```\n\
        ## Build · Linux x86-64\n\
        ### The `ci` configuration: *optimized*?\n\
        <a id=\"custom\"></a>\n";
    assert_eq!(
        anchor(Path::new("document/automation.md"), text),
        BTreeSet::from(
            [
                "build-organization",
                "build--linux-x86-64",
                "build--linux-x86-64-1",
                "the-ci-configuration-optimized",
                "custom",
            ]
            .map(str::to_owned)
        )
    );
    assert_eq!(
        anchor(Path::new("index.html"), text),
        BTreeSet::from(["custom".to_owned()])
    );
}

#[test]
fn missing() {
    let found = BTreeSet::from([
        "//:install".to_owned(),
        "//toolchain/browser:serve".to_owned(),
    ]);
    assert_eq!(package("//:install"), "");
    assert_eq!(package("//toolchain/browser:serve"), "toolchain/browser");
    assert_eq!(package("//library"), "library");
    assert_eq!(dead("//:install", true, &found), None);
    assert_eq!(dead("//toolchain/browser:serve", true, &found), None);
    assert_eq!(dead("//library", true, &found), None);
    assert_eq!(
        dead("//:missing", true, &found),
        Some("is not a Bazel target")
    );
    assert_eq!(dead("//absent", false, &found), Some("is not a package"));
    assert_eq!(
        dead("//absent:target", false, &found),
        Some("is not in a package")
    );
}
