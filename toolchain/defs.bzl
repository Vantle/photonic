"""Portable executable and test rules using HermeticBuild's public launcher API."""

load("@hermetic_launcher//launcher:lib.bzl", "launcher")

def _launch(context):
    suffix = ".exe" if context.target_platform_has_constraint(context.attr._windows[platform_common.ConstraintValueInfo]) else ""
    argument = [context.expand_location(value, targets = [context.attr.entrypoint] + context.attr.data) for value in context.attr.argument]
    output = context.actions.declare_file(context.label.name + suffix)
    extra = []
    dependency = [context.attr.entrypoint] + context.attr.data
    if len(argument) > 9 or "" in argument:
        configuration = context.actions.declare_file(context.label.name + ".json")
        context.actions.write(configuration, json.encode(argument))
        launcher.entrypoint(context.executable._argument).runfiles(context.executable.entrypoint, configuration).compile(context, output_file = output)
        extra = [configuration, context.executable._argument]
        dependency.append(context.attr._argument)
    else:
        launcher.entrypoint(context.executable.entrypoint).embedded_args(*argument).compile(context, output_file = output)
    runfile = context.runfiles(files = context.files.data + [context.executable.entrypoint] + extra)
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
