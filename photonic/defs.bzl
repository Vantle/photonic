"""Native Photonic source libraries and executable programs."""

load("//toolchain:defs.bzl", "hermetic_binary", "hermetic_test")

Info = provider(
    doc = "Transitive declaration sources.",
    fields = {
        "source": "A depset of native declaration files.",
    },
)

def _assemble(context, source, library):
    output = context.actions.declare_file(context.label.name + ".json")
    argument = context.actions.args()
    argument.add("--output", output)
    argument.add_all(source, before_each = "--source")
    argument.add_all(library, before_each = "--library")
    context.actions.run(
        executable = context.executable._assemble,
        arguments = [argument],
        inputs = depset(source + library),
        outputs = [output],
        mnemonic = "Photonic",
        progress_message = "Assembling Photonic %{label}",
    )
    return output

def _library(context):
    source = depset(context.files.srcs, transitive = [dependency[Info].source for dependency in context.attr.deps], order = "postorder")
    output = _assemble(context, [], source.to_list())
    return [
        Info(source = source),
        DefaultInfo(files = depset([output])),
    ]

photonic_library = rule(
    implementation = _library,
    attrs = {
        "srcs": attr.label_list(allow_files = [".particle", ".wave"]),
        "deps": attr.label_list(providers = [Info]),
        "_assemble": attr.label(default = "//photonic:assemble", executable = True, cfg = "exec"),
    },
)

def _runfile(context, file):
    if file.short_path.startswith("../"):
        return file.short_path[3:]
    return context.workspace_name + "/" + file.short_path

def _load(context):
    source = depset(context.files.srcs).to_list()
    library = depset(transitive = [dependency[Info].source for dependency in context.attr.deps], order = "postorder").to_list()
    overlap = {file.path: True for file in library}
    if any([file.path in overlap for file in source]):
        fail("a source cannot also be supplied by a library dependency")
    return _assemble(context, source, library)

def _binary(context):
    output = _load(context)
    return [DefaultInfo(files = depset([output]))]

_program = rule(
    implementation = _binary,
    attrs = {
        "srcs": attr.label_list(allow_files = [".particle", ".wave"], mandatory = True),
        "deps": attr.label_list(providers = [Info]),
        "_assemble": attr.label(default = "//photonic:assemble", executable = True, cfg = "exec"),
    },
)

def photonic_binary(name, srcs, deps = [], visibility = None):
    """Build an executable from N native sources and transitive libraries.

    Args:
        name: Executable target name.
        srcs: Native source files containing initial data or declarations.
        deps: Photonic libraries supplying declarations.
        visibility: Packages allowed to depend on the executable and its assembled program.
    """
    _program(name = name + ".program", srcs = srcs, deps = deps, visibility = visibility)
    hermetic_binary(
        name = name,
        entrypoint = "//photonic:launch",
        argument = ["$(rlocationpath //command:photonic)", "$(rlocationpath :" + name + ".program)"],
        data = [":" + name + ".program", "//command:photonic"],
        visibility = visibility,
    )

def _check(context):
    if context.attr.path and context.attr.expect != "reached":
        fail("a direct path can witness reachability but cannot prove unreachability")
    if any([getattr(context.attr, name) < 0 for name in ["work", "configuration", "occurrence", "scope", "coherence", "record"]]):
        fail("test execution limits must be nonnegative")
    program = _load(context)
    output = context.actions.declare_file(context.label.name + ".case.json")
    context.actions.write(output, json.encode({
        "program": _runfile(context, program),
        "source": context.attr.source,
        "target": context.attr.target,
        "expect": context.attr.expect,
        "path": context.attr.path,
        "work": context.attr.work,
        "limit": {
            "configuration": context.attr.configuration,
            "coherence": context.attr.coherence,
            "occurrence": context.attr.occurrence,
            "scope": context.attr.scope,
            "record": context.attr.record,
        },
    }))
    return [DefaultInfo(files = depset([output]), runfiles = context.runfiles(files = [output, program]))]

_case = rule(
    implementation = _check,
    attrs = {
        "srcs": attr.label_list(allow_files = [".particle", ".wave"]),
        "deps": attr.label_list(providers = [Info]),
        "source": attr.string(),
        "target": attr.string_list(mandatory = True, allow_empty = False),
        "expect": attr.string(mandatory = True, values = ["reached", "unreachable"]),
        "path": attr.bool(mandatory = True),
        "work": attr.int(mandatory = True),
        "configuration": attr.int(mandatory = True),
        "occurrence": attr.int(mandatory = True),
        "scope": attr.int(mandatory = True),
        "coherence": attr.int(mandatory = True),
        "record": attr.int(mandatory = True),
        "_assemble": attr.label(default = "//photonic:assemble", executable = True, cfg = "exec"),
    },
)

def photonic_test(name, target, source = "", srcs = [], deps = [], expect = "reached", path = False, work = 2000000, configuration = 4096, occurrence = 256, scope = 64, coherence = 64, record = 2000000, size = "small", tags = []):
    """Check exact configurations with Prism; Unknown always fails.

    Args:
        name: Test target name.
        target: Configurations in literal Photonic syntax; each also expects every loaded root rule.
        source: Literal Photonic source, including data and declarations.
        srcs: Native source files containing data or declarations.
        deps: Photonic declaration libraries.
        expect: Required reached or unreachable outcome for every target.
        path: Follow one path to witness a reachable target.
        work: Work budget.
        configuration: Configuration limit.
        occurrence: Occurrence limit.
        scope: Scope limit.
        coherence: Coherence limit.
        record: Event record limit.
        size: Bazel test size.
        tags: Bazel test tags.
    """
    _case(name = name + ".case", source = source, target = target, srcs = srcs, deps = deps, expect = expect, path = path, work = work, configuration = configuration, occurrence = occurrence, scope = scope, coherence = coherence, record = record, visibility = ["//visibility:private"], testonly = True)
    hermetic_test(
        name = name,
        entrypoint = "//photonic:check",
        argument = ["$(rlocationpath :" + name + ".case)"],
        data = [":" + name + ".case"],
        size = size,
        tags = tags,
    )
