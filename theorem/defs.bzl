"""Theorems proved, and claims refuted, by executing them."""

load("//photonic:defs.bzl", "photonic_binary", "photonic_test")

def theorem(name, srcs, deps, cells = 256, states = 4096):
    """Prove a claim by reaching exactly Theorem, with every loaded rule.

    Args:
        name: Theorem name; its proof is the test name + ".proof".
        srcs: Sources stating the cases, the claim and the conclusion.
        deps: Libraries defining the operations the claim evaluates.
        cells: Occurrence limit, which grows with the claim and its cases.
        states: Configuration limit; a direct path retains every configuration it visits.
    """
    photonic_binary(name = name, srcs = srcs, deps = deps + ["//theorem:case"])
    photonic_test(
        name = name + ".proof",
        srcs = srcs,
        cells = cells,
        path = True,
        preserve = True,
        states = states,
        targets = ["Theorem"],
        deps = deps + ["//theorem:case"],
    )

def refutation(name, srcs, deps, outcome, cells = 256, states = 4096):
    """Refute a claim by reaching exactly its counterexamples.

    Args:
        name: Claim name; its refutation is the test name + ".refutation".
        srcs: Sources stating the cases, the claim and the conclusion.
        deps: Libraries defining the operations the claim evaluates.
        outcome: The configuration the claim ends in, holding its counterexamples.
        cells: Occurrence limit, which grows with the claim and its cases.
        states: Configuration limit; a direct path retains every configuration it visits.
    """
    photonic_binary(name = name, srcs = srcs, deps = deps + ["//theorem:case"])
    photonic_test(
        name = name + ".refutation",
        srcs = srcs,
        cells = cells,
        path = True,
        preserve = True,
        states = states,
        targets = [outcome],
        deps = deps + ["//theorem:case"],
    )
