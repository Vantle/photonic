"""Check the declared Starlark inventory with the pinned Buildifier toolchain."""

def _check(context):
    output = context.actions.declare_file(context.label.name + ".buildifier.ok")
    executable = context.toolchains["@buildifier_prebuilt//buildifier:toolchain"]._tool
    argument = context.actions.args()
    argument.add(output)
    argument.add(executable)
    argument.add_all(["-mode=check", "-lint=warn"])
    argument.add_all(context.files.source)
    context.actions.run(
        executable = context.executable._verify,
        arguments = [argument],
        inputs = context.files.source,
        tools = [executable],
        outputs = [output],
        mnemonic = "Buildifier",
        progress_message = "Buildifier %{label}",
    )
    return [DefaultInfo(files = depset([output]))]

starlark = rule(
    implementation = _check,
    attrs = {
        "source": attr.label(mandatory = True, allow_files = True),
        "_verify": attr.label(default = "//toolchain:verify", executable = True, cfg = "exec"),
    },
    toolchains = ["@buildifier_prebuilt//buildifier:toolchain"],
)
