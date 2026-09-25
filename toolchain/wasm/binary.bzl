"""Expose a pinned executable file to a toolchain."""

def _binary(context):
    output = context.actions.declare_file(context.label.name)
    context.actions.symlink(output = output, target_file = context.file.source, is_executable = True)
    return [DefaultInfo(executable = output)]

binary = rule(
    implementation = _binary,
    attrs = {"source": attr.label(allow_single_file = True, mandatory = True)},
    executable = True,
)
