"""Expose a pinned executable file to a toolchain."""

def _binary(ctx):
    output = ctx.actions.declare_file(ctx.label.name)
    ctx.actions.symlink(output = output, target_file = ctx.file.source, is_executable = True)
    return [DefaultInfo(executable = output)]

binary = rule(
    implementation = _binary,
    attrs = {"source": attr.label(allow_single_file = True, mandatory = True)},
    executable = True,
)
