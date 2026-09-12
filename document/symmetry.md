# Coherence symmetry

Implemented general canonicalization optimization. The rule application, source-inference, and ownership semantics are unchanged. This is not an arithmetic algorithm.

## Swap certificate

Two coherences may be interchanged by this optimization only when they have the same frame and the same resource description. Occurrences used outside a coherence, including held environments, retain their exact resource identity in that description. Resources confined to the coherence are compared by value, capture, and occurrence multiplicity, allowing anonymous renaming.

This certifies an automorphism fixing the surrounding state: swap the two coherences and consistently rename their private resources; shared resources, frame identities, and capture references remain fixed. Every certified swap preserves the state. The certificate is sufficient, not a complete automorphism algorithm. More complex symmetries remain in the ordinary search.

The ordering cursor enumerates only arrangements that retain increasing original indices within each certified class. It preserves the lexicographic order of that subset of the original exhaustive enumeration. Each omitted ordering has an earlier retained representative producing the same canonical state. This preserves the existing minimum and tie choice, including the renaming map used by provenance. It does not merge the coherences themselves or equate independent introductions with shared ones.

The optimization is attempted for configurations with at least four coherences and an unresolved refinement class. Smaller configurations use the ordinary permutation cursor because certificate construction can outweigh the saved work. This is an execution-cost threshold with identical meaning on either side, not a language rule or arithmetic special case. The iterator uses ordinary lexicographic permutation when there is no certified equivalence to exploit.

## Reproduce

```sh
bazel run -c opt //system:symmetry
bazel test //system:ordering //system:test
```

The benchmark measures canonicalization construction, incremental search, and extraction of a completed result. Input allocation happens before timing. Each case has one warm-up and 25 samples, with a 1,000-step search budget. Medians below are microseconds on the development host; incomplete rows time only the bounded prefix, not a completed normalization. [Raw measurements](symmetry.json).

| Sharing | Coherences | Search steps | Complete | Median µs |
| --- | ---: | ---: | --- | ---: |
| private | 2 | 5 | true | 4.54 |
| private | 4 | 3 | true | 3.42 |
| private | 8 | 3 | true | 5.33 |
| private | 30 | 3 | true | 15.58 |
| shared | 2 | 5 | true | 2.08 |
| shared | 4 | 3 | true | 3.08 |
| shared | 8 | 3 | true | 4.58 |
| shared | 30 | 3 | true | 11.67 |
| ring | 2 | 5 | true | 2.96 |
| ring | 4 | 49 | true | 28.54 |
| ring | 8 | 1000 | false | 750.17 |
| ring | 30 | 1000 | false | 1849.92 |

Eight independent, identically shaped coherences previously required 8! = 40,320 world orderings under the exhaustive cursor. The certificate reduces this case to one ordering. Thirty interchangeable coherences complete in three search steps; a 30-coherence sharing ring still suspends at 1,000 steps. Thus this removes one important source of factorial work without claiming to solve general graph symmetry.

## Verification and limits

An independent permutation oracle exhaustively checks every three-class assignment for up to five indices, including partitioned orderings and exact enumeration order. Runtime tests retain all thirty resources for independent worlds, one resource shared across thirty worlds, and their unequal states. An additional 512 sharing graphs check invariance under world permutation and resource renaming. Existing tests compare complete semantic graphs and deterministic execution across worker counts.

A sequential before/after run of the 19 small reference benchmarks is recorded in [the comparison](symmetry-comparison.json). The median of the 19 per-program current/baseline latency ratios was approximately 1.10 in that sequential comparison. Those inputs are mostly too small or asymmetric to benefit substantially; timing variation and certificate overhead can dominate. This is not evidence of an overall application speedup. The targeted search-step reduction is the demonstrated improvement. Broader benchmarks and more efficient certificates remain work.

Parallel worlds remain distinct execution contexts. The existing worker executor can advance independent search and normalization jobs concurrently. This change improves each normalization job; it neither serializes the language nor establishes that arbitrary rules have disjoint effects.
