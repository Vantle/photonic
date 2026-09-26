"""Theorems proved, and claims refuted, by executing them."""

load("//photonic:defs.bzl", "photonic_binary", "photonic_test")

def theorem(name, srcs, deps, occurrence = 256, configuration = 4096, path = True):
    """Prove a claim by reaching exactly Theorem, with every loaded rule.

    Args:
        name: Theorem name; its proof is the test name + ".proof".
        srcs: Sources stating the cases, the claim and the conclusion.
        deps: Libraries the claim loads: the operations it evaluates, and //theorem:case when it
            states cases.
        occurrence: Occurrence limit, which grows with the claim and its cases.
        configuration: Configuration limit; a direct path retains every configuration it visits.
        path: Follow one direct execution, which suffices when every execution reaches Theorem;
            otherwise Prism explores every order of the rules.
    """
    photonic_binary(name = name, srcs = srcs, deps = deps)
    photonic_test(
        name = name + ".proof",
        srcs = srcs,
        occurrence = occurrence,
        path = path,
        configuration = configuration,
        target = ["Theorem"],
        deps = deps,
    )

def refutation(name, srcs, deps, outcome, occurrence = 256, configuration = 4096):
    """Refute a claim by reaching exactly its counterexamples.

    Args:
        name: Claim name; its refutation is the test name + ".refutation".
        srcs: Sources stating the cases, the claim and the conclusion.
        deps: Libraries the claim loads, including //theorem:case.
        outcome: The configuration the claim ends in, holding its counterexamples.
        occurrence: Occurrence limit, which grows with the claim and its cases.
        configuration: Configuration limit; a direct path retains every configuration it visits.
    """
    photonic_binary(name = name, srcs = srcs, deps = deps)
    photonic_test(
        name = name + ".refutation",
        srcs = srcs,
        occurrence = occurrence,
        path = True,
        configuration = configuration,
        target = [outcome],
        deps = deps,
    )
