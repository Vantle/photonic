"""Native Photonic source libraries and executable programs."""

load("@bazel_skylib//rules:native_binary.bzl", "native_test")
load("@rules_rs//rs:rust_binary.bzl", "rust_binary")

Info = provider(
    doc = "Transitive declaration sources and validated assembly artifacts.",
    fields = {
        "source": "A depset of native declaration files.",
        "validation": "A depset of validated library artifacts.",
    },
)

def _assemble(ctx, source, library, validation):
    output = ctx.actions.declare_file(ctx.label.name + ".json")
    argument = ctx.actions.args()
    argument.add("--output", output)
    argument.add_all(source, before_each = "--source")
    argument.add_all(library, before_each = "--library")
    ctx.actions.run(
        executable = ctx.executable._assemble,
        arguments = [argument],
        inputs = depset(source + library, transitive = [validation]),
        outputs = [output],
        mnemonic = "Photonic",
        progress_message = "Assembling Photonic %{label}",
    )
    return output

def _library(ctx):
    source = depset(ctx.files.srcs, transitive = [dep[Info].source for dep in ctx.attr.deps], order = "postorder")
    validation = depset(transitive = [dep[Info].validation for dep in ctx.attr.deps])
    output = _assemble(ctx, [], source.to_list(), validation)
    return [
        Info(source = source, validation = depset([output], transitive = [validation])),
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

def _runfile(ctx, file):
    if file.short_path.startswith("../"):
        return file.short_path[3:]
    return ctx.workspace_name + "/" + file.short_path

def _load(ctx):
    source = depset(ctx.files.srcs).to_list()
    library = depset(transitive = [dep[Info].source for dep in ctx.attr.deps], order = "postorder").to_list()
    overlap = {file.path: True for file in library}
    if any([file.path in overlap for file in source]):
        fail("a source cannot also be supplied by a library dependency")
    validation = depset(transitive = [dep[Info].validation for dep in ctx.attr.deps])
    return _assemble(ctx, source, library, validation)

def _binary(ctx):
    output = _load(ctx)
    launcher = ctx.actions.declare_file(ctx.label.name + ".rs")
    ctx.actions.write(launcher, "const EXECUTABLE: &str = %s;\nconst SOURCE: &str = %s;\n\nfn main() -> Result<std::process::ExitCode, std::io::Error> {\n    launch::run(EXECUTABLE, SOURCE)\n}\n" % (
        json.encode(_runfile(ctx, ctx.executable._command)),
        json.encode(_runfile(ctx, output)),
    ))
    return [
        DefaultInfo(files = depset([output, launcher])),
        OutputGroupInfo(source = depset([launcher]), program = depset([output])),
    ]

_program = rule(
    implementation = _binary,
    attrs = {
        "srcs": attr.label_list(allow_files = [".particle", ".wave"], mandatory = True),
        "deps": attr.label_list(providers = [Info]),
        "_assemble": attr.label(default = "//photonic:assemble", executable = True, cfg = "exec"),
        "_command": attr.label(default = "//system:command", executable = True, cfg = "target"),
    },
)

def photonic_binary(name, srcs, deps = [], visibility = None, testonly = False, tags = []):
    """Build an executable from N native sources and transitive libraries.

    Args:
        name: Executable target name.
        srcs: Native source files containing initial data or declarations.
        deps: Photonic libraries supplying declarations.
        visibility: Packages allowed to depend on the executable.
        testonly: Whether this target is restricted to tests.
        tags: Bazel tags attached to the executable.
    """
    _program(name = name + ".assembly", srcs = srcs, deps = deps, visibility = ["//visibility:private"], testonly = testonly)
    native.filegroup(name = name + ".source", srcs = [":" + name + ".assembly"], output_group = "source", visibility = ["//visibility:private"], testonly = testonly)
    native.filegroup(name = name + ".program", srcs = [":" + name + ".assembly"], output_group = "program", visibility = visibility, testonly = testonly)
    rust_binary(
        name = name,
        srcs = [":" + name + ".source"],
        crate_root = ":" + name + ".source",
        crate_name = "program",
        deps = ["//photonic:launch"],
        data = [":" + name + ".program", "//system:command"],
        visibility = visibility,
        testonly = testonly,
        tags = tags,
    )

def _check(ctx):
    if ctx.attr.path and ctx.attr.expect != "reached":
        fail("a direct path can witness reachability but cannot prove unreachability")
    if any([getattr(ctx.attr, name) < 0 for name in ["steps", "states", "cells", "frames", "coherences", "records"]]):
        fail("test execution limits must be nonnegative")
    program = _load(ctx)
    output = ctx.actions.declare_file(ctx.label.name + ".case.json")
    ctx.actions.write(output, json.encode({
        "program": _runfile(ctx, program),
        "source": ctx.attr.source,
        "targets": ctx.attr.targets,
        "expect": ctx.attr.expect,
        "match": ctx.attr.match,
        "path": ctx.attr.path,
        "step": ctx.attr.steps,
        "state": ctx.attr.states,
        "cell": ctx.attr.cells,
        "frame": ctx.attr.frames,
        "coherence": ctx.attr.coherences,
        "record": ctx.attr.records,
    }))
    return [DefaultInfo(files = depset([output]), runfiles = ctx.runfiles(files = [output, program]))]

_case = rule(
    implementation = _check,
    attrs = {
        "srcs": attr.label_list(allow_files = [".particle", ".wave"]),
        "deps": attr.label_list(providers = [Info]),
        "source": attr.string(),
        "targets": attr.string_list(mandatory = True, allow_empty = False),
        "match": attr.string(default = "all", values = ["all", "any"]),
        "expect": attr.string(default = "reached", values = ["reached", "unreachable"]),
        "path": attr.bool(default = False),
        "steps": attr.int(default = 2000000),
        "states": attr.int(default = 4096),
        "cells": attr.int(default = 256),
        "frames": attr.int(default = 64),
        "coherences": attr.int(default = 64),
        "records": attr.int(default = 2000000),
        "_assemble": attr.label(default = "//photonic:assemble", executable = True, cfg = "exec"),
    },
)

def photonic_test(name, source, targets, srcs = [], deps = [], match = "all", expect = "reached", path = False, steps = 2000000, states = 4096, cells = 256, frames = 64, coherences = 64, records = 2000000, size = "small", visibility = None, tags = []):
    """Check an exact configuration with Obsidian; Unknown always fails.

    Args:
        name: Test target name.
        source: Literal Photonic source, including data and declarations.
        targets: Accepted configurations in literal Photonic syntax.
        srcs: Native source files containing data or declarations.
        deps: Photonic declaration libraries.
        match: Require all targets or any target to satisfy the expectation.
        expect: Required reached or unreachable outcome for each target.
        path: Follow one path to witness a reachable target.
        steps: Work budget.
        states: Configuration limit.
        cells: Occurrence limit.
        frames: Scope limit.
        coherences: Coherence limit.
        records: Event record limit.
        size: Bazel test size.
        visibility: Packages allowed to depend on the test.
        tags: Bazel test tags.
    """
    _case(name = name + ".case", source = source, targets = targets, srcs = srcs, deps = deps, match = match, expect = expect, path = path, steps = steps, states = states, cells = cells, frames = frames, coherences = coherences, records = records, visibility = ["//visibility:private"], testonly = True)
    native_test(
        name = name,
        src = "//photonic:check",
        args = ["$(rlocationpath :" + name + ".case)"],
        data = [":" + name + ".case"],
        size = size,
        visibility = visibility,
        tags = tags,
    )
