"""Portable executable and test rules using HermeticBuild's public launcher API."""

load("@hermetic_launcher//launcher:lib.bzl", "launcher")

def _compile(context, entrypoint, argument, data):
    suffix = ".exe" if context.target_platform_has_constraint(context.attr._windows[platform_common.ConstraintValueInfo]) else ""
    executable = entrypoint[DefaultInfo].files_to_run.executable
    output = context.actions.declare_file(context.label.name + suffix)
    extra = []
    dependency = [entrypoint] + data
    if len(argument) > 9 or "" in argument:
        configuration = context.actions.declare_file(context.label.name + ".json")
        context.actions.write(configuration, json.encode(argument))
        launcher.entrypoint(context.executable._argument).runfiles(executable, configuration).compile(context, output_file = output)
        extra = [configuration, context.executable._argument]
        dependency.append(context.attr._argument)
    else:
        launcher.entrypoint(executable).embedded_args(*argument).compile(context, output_file = output)
    file = depset(transitive = [target[DefaultInfo].files for target in data])
    runfile = context.runfiles(files = [executable] + extra, transitive_files = file)
    runfile = runfile.merge_all([target[DefaultInfo].default_runfiles for target in dependency])
    return [DefaultInfo(executable = output, runfiles = runfile)]

def _launch(context):
    argument = [context.expand_location(value, targets = [context.attr.entrypoint] + context.attr.data) for value in context.attr.argument]
    return _compile(context, context.attr.entrypoint, argument, context.attr.data)

def _path(target):
    file = target[DefaultInfo].files.to_list()
    if len(file) != 1:
        fail("{} must provide one file to pass as a path".format(target.label))
    return launcher.to_rlocation_path(file[0])

def _interpret(context):
    path = [launcher.to_rlocation_path(file) for file in [context.executable._node, context.file.script]] + [_path(target) for target in context.attr.runfile]
    separator = ["--"] if context.attr.argument else []
    data = [context.attr._node, context.attr.script] + context.attr.runfile + context.attr.data
    return _compile(context, context.attr._execute, path + separator + context.attr.argument, data)

def _attribute(value):
    return value | {
        "argument": attr.string_list(),
        "data": attr.label_list(allow_files = True),
        "_argument": attr.label(default = "//toolchain:argument", executable = True, cfg = "target"),
        "_windows": attr.label(default = "@platforms//os:windows"),
    }

_HERMETIC = _attribute({
    "entrypoint": attr.label(executable = True, cfg = "target", mandatory = True),
})

_SCRIPT = _attribute({
    "script": attr.label(allow_single_file = [".mjs"], mandatory = True),
    "runfile": attr.label_list(allow_files = True),
    "_execute": attr.label(default = "//toolchain:execute", executable = True, cfg = "target"),
    "_node": attr.label(default = "//toolchain:node", executable = True, cfg = "target"),
})

_TOOLCHAIN = [launcher.template_toolchain_type, launcher.finalizer_toolchain_type]

hermetic_binary = rule(
    implementation = _launch,
    executable = True,
    attrs = _HERMETIC,
    toolchains = _TOOLCHAIN,
)

hermetic_test = rule(
    implementation = _launch,
    test = True,
    attrs = _HERMETIC,
    toolchains = _TOOLCHAIN,
)

script_binary = rule(
    implementation = _interpret,
    executable = True,
    attrs = _SCRIPT,
    toolchains = _TOOLCHAIN,
)

script_test = rule(
    implementation = _interpret,
    test = True,
    attrs = _SCRIPT,
    toolchains = _TOOLCHAIN,
)
