"""Expose the pinned Node runtime as a native executable."""

def _node(context):
    runtime = context.toolchains["@rules_nodejs//nodejs:runtime_toolchain_type"].nodeinfo.node
    suffix = ".exe" if context.target_platform_has_constraint(context.attr._windows[platform_common.ConstraintValueInfo]) else ""
    output = context.actions.declare_file(context.label.name + suffix)
    context.actions.symlink(output = output, target_file = runtime, is_executable = True)
    return [DefaultInfo(executable = output, runfiles = context.runfiles(files = [runtime]))]

node = rule(
    implementation = _node,
    executable = True,
    attrs = {"_windows": attr.label(default = "@platforms//os:windows")},
    toolchains = ["@rules_nodejs//nodejs:runtime_toolchain_type"],
)
