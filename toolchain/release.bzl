"""The optimized build of an executable, whatever compilation mode the command line asks for."""

def _optimize(_setting, _attribute):
    return {"//command_line_option:compilation_mode": "opt"}

_optimized = transition(
    implementation = _optimize,
    inputs = [],
    outputs = ["//command_line_option:compilation_mode"],
)

def _release(context):
    actual = context.attr.actual[0][DefaultInfo]
    return [DefaultInfo(
        files = depset([actual.files_to_run.executable]),
        runfiles = actual.default_runfiles,
    )]

release = rule(
    implementation = _release,
    attrs = {
        "actual": attr.label(cfg = _optimized, executable = True, mandatory = True),
    },
)
