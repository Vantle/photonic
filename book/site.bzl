"""Assemble the published webbook into one directory."""

REPOSITORY = "https://github.com/Vantle/photonic"

def _site(context):
    output = context.actions.declare_directory(context.label.name)
    node = context.toolchains["@rules_nodejs//nodejs:toolchain_type"].nodeinfo.node
    argument = context.actions.args()
    argument.add(context.file.script)
    argument.add(output.path)
    argument.add(context.attr.repository)
    for file in context.files.srcs:
        argument.add(file.path)
        argument.add(file.short_path)
    context.actions.run(
        executable = node,
        arguments = [argument],
        inputs = [context.file.script] + context.files.srcs,
        outputs = [output],
        mnemonic = "Site",
        progress_message = "Assembling the webbook site %{label}",
    )
    return [DefaultInfo(files = depset([output]))]

site = rule(
    implementation = _site,
    attrs = {
        "repository": attr.string(mandatory = True),
        "script": attr.label(allow_single_file = [".mjs"], mandatory = True),
        "srcs": attr.label_list(allow_files = True, mandatory = True),
    },
    toolchains = ["@rules_nodejs//nodejs:toolchain_type"],
)
