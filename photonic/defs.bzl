"""Native Photonic source libraries, executable programs and tests."""

load("//toolchain:defs.bzl", "hermetic_binary", "hermetic_test")

Info = provider(
    doc = "Transitive declaration sources.",
    fields = {
        "source": "A depset of native declaration files.",
    },
)

def _assemble(context, source, library, check = []):
    output = context.actions.declare_file(context.label.name + ".json")
    argument = context.actions.args()
    argument.add("--output", output)
    argument.add_all(source, before_each = "--source")
    argument.add_all(library, before_each = "--library")
    argument.add_all(check, before_each = "--check")
    context.actions.run(
        executable = context.executable._assemble,
        arguments = [argument],
        inputs = depset(source + library + check),
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
    doc = "A library of Photonic declarations. Its output is the assembled library, name + \".json\", which the command line's --library also accepts.",
    attrs = {
        "srcs": attr.label_list(allow_files = [".particle", ".wave"], doc = "Declaration files; each holds rules only."),
        "deps": attr.label_list(providers = [Info], doc = "Libraries these declarations build on, loaded first."),
        "_assemble": attr.label(default = Label("//photonic:assemble"), executable = True, cfg = "exec"),
    },
)

def _runfile(context, file):
    if file.short_path.startswith("../"):
        return file.short_path[3:]
    return context.workspace_name + "/" + file.short_path

def _load(context, check = []):
    source = depset(context.files.srcs).to_list()
    library = depset(transitive = [dependency[Info].source for dependency in context.attr.deps], order = "postorder").to_list()
    overlap = {file.path: True for file in library}
    if any([file.path in overlap for file in source]):
        fail("a source cannot also be supplied by a library dependency")
    return _assemble(context, source, library, check)

def _binary(context):
    output = _load(context)
    return [DefaultInfo(files = depset([output]))]

_program = rule(
    implementation = _binary,
    attrs = {
        "srcs": attr.label_list(allow_files = [".particle", ".wave"], mandatory = True),
        "deps": attr.label_list(providers = [Info]),
        "_assemble": attr.label(default = Label("//photonic:assemble"), executable = True, cfg = "exec"),
    },
)

def photonic_binary(name, srcs, deps = [], visibility = None):
    """Build an executable from native sources and transitive libraries.

    The executable runs the photonic command on the assembled program, name + ".program": run by
    default, or the verb given as its first argument, such as check or explore.

    Args:
        name: Executable target name.
        srcs: Native source files containing initial data or declarations.
        deps: Photonic libraries supplying declarations.
        visibility: Packages allowed to depend on the executable and its assembled program; both
            stay private to the package unless it is given.
    """
    _program(name = name + ".program", srcs = srcs, deps = deps, visibility = visibility)
    hermetic_binary(
        name = name,
        entrypoint = Label("//photonic:launch"),
        argument = ["$(rlocationpath :{}.program)".format(name)],
        data = [":{}.program".format(name)],
        visibility = visibility,
    )

# Each source and target is written to a file of its own and parsed when the case is assembled, so
# a syntax error fails the build and names the attribute that holds it.
def _text(context, name, content):
    file = context.actions.declare_file("{}.{}.wave".format(context.label.name, name))
    context.actions.write(file, content)
    return file

def _check(context):
    if context.attr.path and context.attr.expect != "reached":
        fail("a direct path can witness reachability but cannot prove unreachability")
    if context.attr.every and (context.attr.path or context.attr.expect != "reached"):
        fail("every schedule ends at a target explores every plain schedule and expects reached")
    if any([getattr(context.attr, name) < 0 for name in ["work", "configuration", "occurrence", "scope", "coherence", "record"]]):
        fail("test execution limits must be nonnegative")
    text = [_text(context, "source", context.attr.source)] + [
        _text(context, "target.{}".format(index), target)
        for index, target in enumerate(context.attr.target)
    ]
    program = _load(context, text)
    output = context.actions.declare_file(context.label.name + ".case.json")
    context.actions.write(output, json.encode({
        "program": _runfile(context, program),
        "source": context.attr.source,
        "target": context.attr.target,
        "expect": context.attr.expect,
        "path": context.attr.path,
        "every": context.attr.every,
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
        "every": attr.bool(mandatory = True),
        "work": attr.int(mandatory = True),
        "configuration": attr.int(mandatory = True),
        "occurrence": attr.int(mandatory = True),
        "scope": attr.int(mandatory = True),
        "coherence": attr.int(mandatory = True),
        "record": attr.int(mandatory = True),
        "_assemble": attr.label(default = Label("//photonic:assemble"), executable = True, cfg = "exec"),
    },
)

def photonic_test(name, target, source = "", srcs = [], deps = [], expect = "reached", path = False, every = False, work = 2000000, configuration = 4096, occurrence = 256, scope = 64, coherence = 64, record = 2000000, size = "small", tags = []):
    """Check that a program reaches, or never reaches, exact configurations; Unknown always fails.

    A test runs in one of three modes. By default Prism explores every future, with inference, on
    the interpreter and on Laser, which must agree wherever both settle: this proves reached and
    unreachable alike, but it finishes only on small programs, such as calls to the scalar tables.
    Linked data, such as chains, naturals, vectors, expressions and pipelines, needs one of the
    other modes: path = True follows the one run the scheduler takes, and every = True checks every
    schedule of plain events. A failing test prints what the search found and how far it went, and
    a photonic check command that asks the same question; its undeclared outputs keep the details.

    Args:
        name: Test target name.
        target: Configurations in literal Photonic syntax; each also expects every loaded root rule.
        source: Literal Photonic source, including data and declarations.
        srcs: Native source files containing data or declarations.
        deps: Photonic declaration libraries.
        expect: Required reached or unreachable outcome for every target.
        path: Follow one direct path per target, the run the scheduler takes, which witnesses a
            reachable target; it cannot expect unreachable.
        every: Require every schedule of plain events to end exactly at a target, exploring the
            schedules without inference on Laser, one commuting coherence or scope at a time; no run
            may go on forever. It expects reached and cannot follow a path.
        work: Work budget of each engine that runs: the interpreter's steps, the path search's
            steps, and Laser's scanned configurations, carried traces and tried applications. The
            engines count work differently, so one budget stops them at different points.
        configuration: Configuration limit.
        occurrence: Occurrence limit: the occurrences one configuration's coherences and scopes hold
            as values, not counting live rules.
        scope: Scope limit: the scopes one configuration has opened, not counting the root.
        coherence: Coherence limit.
        record: Record budget: the records each engine retains.
        size: Bazel test size.
        tags: Bazel test tags, such as memory for a test that needs more than 2 GiB.
    """
    _case(name = name + ".case", source = source, target = target, srcs = srcs, deps = deps, expect = expect, path = path, every = every, work = work, configuration = configuration, occurrence = occurrence, scope = scope, coherence = coherence, record = record, visibility = ["//visibility:private"], testonly = True)
    hermetic_test(
        name = name,
        entrypoint = Label("//photonic:check"),
        argument = ["$(rlocationpath :{}.case)".format(name)],
        data = [":{}.case".format(name)],
        size = size,
        tags = tags,
    )
