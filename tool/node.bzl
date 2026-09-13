"""Expose the pinned Node runtime as a native executable."""

def _node(ctx):
    runtime = ctx.toolchains["@rules_nodejs//nodejs:runtime_toolchain_type"].nodeinfo.node
    output = ctx.actions.declare_file(ctx.label.name + ".exe")
    ctx.actions.symlink(output = output, target_file = runtime, is_executable = True)
    return [DefaultInfo(executable = output, runfiles = ctx.runfiles(files = [runtime]))]

node = rule(
    implementation = _node,
    executable = True,
    toolchains = ["@rules_nodejs//nodejs:runtime_toolchain_type"],
)
