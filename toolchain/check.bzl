"""Propagate Rust checks through source, executable, and WebAssembly dependencies."""

load("@rules_rust//rust:defs.bzl", "rust_clippy_aspect", "rustfmt_aspect")

def _collect(target):
    if OutputGroupInfo not in target:
        return []
    output = target[OutputGroupInfo]
    return [getattr(output, name) for name in ["clippy_checks", "rustfmt_checks", "check"] if hasattr(output, name)]

def _check(target, context):
    if not target.label.workspace_root and hasattr(context.rule.attr, "lint_config") and not context.rule.attr.lint_config:
        fail("Rust targets must set lint_config = \"//:lint\"")
    dependency = []
    for name in dir(context.rule.attr):
        value = getattr(context.rule.attr, name)
        if type(value) == "Target":
            dependency.append(value)
        elif type(value) == "list":
            dependency.extend([entry for entry in value if type(entry) == "Target"])
    return [OutputGroupInfo(check = depset(transitive = _collect(target) + [output for entry in dependency for output in _collect(entry)]))]

check = aspect(
    implementation = _check,
    attr_aspects = ["*"],
    requires = [rust_clippy_aspect, rustfmt_aspect],
)
