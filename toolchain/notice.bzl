"""The third-party notices for the crates a binary links."""

Info = provider(
    doc = "The third-party crates a target links, with the files that state their licenses.",
    fields = {
        "crate": "A depset of structs with name, version, manifest and license (a tuple of files).",
    },
)

_PREFIX = ("LICENSE", "LICENCE", "COPYING", "COPYRIGHT", "NOTICE", "UNLICENSE")

# Build scripts and procedural macros run while compiling and leave no code in the binary, so the
# walk follows only the libraries a target links.
_LINKED = ("rust_binary", "rust_library", "rust_test")

def _visit(target, context):
    if context.rule.kind not in _LINKED:
        return [Info(crate = depset())]
    transitive = [dependency[Info].crate for dependency in getattr(context.rule.attr, "deps", []) if Info in dependency]
    if not target.label.repo_name.startswith("rules_rs++crate+crate__"):
        return [Info(crate = depset(transitive = transitive))]
    file = context.rule.files.compile_data
    entry = struct(
        name = context.rule.attr.crate_name or target.label.name,
        version = context.rule.attr.version,
        manifest = tuple([value for value in file if value.short_path.endswith("/Cargo.toml") and value.short_path.count("/") == 2]),
        license = tuple([value for value in file if value.basename.upper().startswith(_PREFIX)]),
    )
    return [Info(crate = depset([entry], transitive = transitive))]

_notice = aspect(
    implementation = _visit,
    attr_aspects = ["deps"],
)

def _render(context):
    crate = depset(transitive = [target[Info].crate for target in context.attr.target]).to_list()
    output = context.actions.declare_file(context.label.name + ".txt")
    argument = context.actions.args()
    argument.add("--output", output)
    input = []
    for entry in sorted(crate, key = lambda value: (value.name, value.version)):
        argument.add("--crate", "{} {}".format(entry.name, entry.version))
        argument.add_all(entry.manifest, before_each = "--manifest")
        argument.add_all(entry.license, before_each = "--license")
        input.extend(entry.manifest)
        input.extend(entry.license)
    argument.add_all(context.files.runtime, before_each = "--runtime")
    input.extend(context.files.runtime)
    context.actions.run(
        executable = context.executable._render,
        arguments = [argument],
        inputs = input,
        outputs = [output],
        mnemonic = "Notice",
        progress_message = "Writing third-party notices for %{label}",
    )
    return [DefaultInfo(files = depset([output]))]

notice = rule(
    implementation = _render,
    doc = "Writes the license expression and license texts of every third-party crate the targets link.",
    attrs = {
        "target": attr.label_list(aspects = [_notice], mandatory = True),
        "runtime": attr.label_list(allow_files = True, doc = "Notices of runtime libraries the toolchain links statically, appended after the crates."),
        "_render": attr.label(default = "//toolchain:notice", executable = True, cfg = "exec"),
    },
)
