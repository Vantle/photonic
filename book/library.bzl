"""Write the standard library into one script, each library with its dependencies in load order."""

load("//photonic:defs.bzl", "Info")

def _stem(file):
    return file.short_path.rsplit(".", 1)[0]

def _pair(file):
    return [_stem(file), file.path]

def _library(context):
    output = context.actions.declare_file(context.label.name + ".js")
    manifest = context.actions.declare_file(context.label.name + ".json")
    context.actions.write(manifest, json.encode([
        {
            "name": "{}/{}".format(target.label.package, target.label.name),
            "load": [_stem(file) for file in target[Info].source.to_list()],
        }
        for target in context.attr.deps
    ]))
    source = depset(transitive = [target[Info].source for target in context.attr.deps])
    argument = context.actions.args()
    argument.add(context.file._script)
    argument.add(output)
    argument.add(manifest)
    argument.add_all(source, map_each = _pair)
    context.actions.run(
        executable = context.toolchains["@rules_nodejs//nodejs:toolchain_type"].nodeinfo.node,
        arguments = [argument],
        inputs = depset([context.file._script, manifest], transitive = [source]),
        outputs = [output],
        mnemonic = "Library",
        progress_message = "Recording the standard library %{label}",
    )
    return [DefaultInfo(files = depset([output]))]

library = rule(
    implementation = _library,
    doc = "The libraries a reader can load in the Lightbox, as book.library: each one's package, source and the libraries it loads first.",
    attrs = {
        "deps": attr.label_list(providers = [Info], mandatory = True, doc = "Library targets, each holding one file named after it."),
        "_script": attr.label(default = ":library.mjs", allow_single_file = [".mjs"]),
    },
    toolchains = ["@rules_nodejs//nodejs:toolchain_type"],
)
