"""The direct path every evaluator case follows, with the budget it shares."""

load("//photonic:defs.bzl", "photonic_test")

def evaluation(name, target, deps, tags = []):
    """Follow the direct path of name + ".wave" to its target.

    Args:
        name: Test name, and the stem of the source it runs.
        target: Configurations the path must reach.
        deps: Photonic libraries the source calls.
        tags: Bazel test tags.
    """
    photonic_test(
        name = name,
        srcs = [name + ".wave"],
        coherence = 1024,
        configuration = 65536,
        occurrence = 16384,
        path = True,
        record = 100000000,
        scope = 2048,
        tags = tags,
        target = target,
        work = 100000000,
        deps = deps,
    )
