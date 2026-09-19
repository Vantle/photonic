"""Portable executable and test rules using HermeticBuild's public launcher API."""

load("@hermetic_launcher//launcher:lib.bzl", "launcher")

def _launch(ctx):
    suffix = ".exe" if ctx.target_platform_has_constraint(ctx.attr._windows[platform_common.ConstraintValueInfo]) else ""
    argument = [ctx.expand_location(value, targets = [ctx.attr.entrypoint] + ctx.attr.data) for value in ctx.attr.argument]
    output = ctx.actions.declare_file(ctx.label.name + suffix)
    extra = []
    dependency = [ctx.attr.entrypoint] + ctx.attr.data
    if len(argument) > 9 or "" in argument:
        configuration = ctx.actions.declare_file(ctx.label.name + ".json")
        ctx.actions.write(configuration, json.encode(argument))
        launcher.entrypoint(ctx.executable._argument).runfiles(ctx.executable.entrypoint, configuration).compile(ctx, output_file = output)
        extra = [configuration, ctx.executable._argument]
        dependency.append(ctx.attr._argument)
    else:
        launcher.entrypoint(ctx.executable.entrypoint).embedded_args(*argument).compile(ctx, output_file = output)
    runfile = ctx.runfiles(files = ctx.files.data + [ctx.executable.entrypoint] + extra)
    runfile = runfile.merge_all([target[DefaultInfo].default_runfiles for target in dependency])
    return [DefaultInfo(executable = output, runfiles = runfile)]

def _attribute():
    return {
        "entrypoint": attr.label(executable = True, cfg = "target", mandatory = True),
        "argument": attr.string_list(),
        "data": attr.label_list(allow_files = True),
        "_argument": attr.label(default = "//toolchain:argument", executable = True, cfg = "target"),
        "_windows": attr.label(default = "@platforms//os:windows"),
    }

hermetic_binary = rule(
    implementation = _launch,
    executable = True,
    attrs = _attribute(),
    toolchains = [launcher.template_toolchain_type, launcher.finalizer_toolchain_type],
)

hermetic_test = rule(
    implementation = _launch,
    test = True,
    attrs = _attribute(),
    toolchains = [launcher.template_toolchain_type, launcher.finalizer_toolchain_type],
)
