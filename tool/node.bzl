"""Expose the pinned Node runtime as a native executable."""

def _node(context):
    runtime = context.toolchains["@rules_nodejs//nodejs:runtime_toolchain_type"].nodeinfo.node
    output = context.actions.declare_file(context.label.name + ".exe")
    context.actions.symlink(output = output, target_file = runtime, is_executable = True)
    return [DefaultInfo(executable = output, runfiles = context.runfiles(files = [runtime]))]

node = rule(
    implementation = _node,
    executable = True,
    toolchains = ["@rules_nodejs//nodejs:runtime_toolchain_type"],
)
