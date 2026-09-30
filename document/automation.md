# Continuous verification

Two services verify the repository. [Photonic on Buildkite](https://buildkite.com/vantle-labs/photonic) runs two independent jobs for every commit to this repository: **Build · Linux x86-64** and **Test · Linux x86-64**. Both select Bazel's `ci` configuration: optimized compilation with `//platform:x86_64-unknown-linux-gnu` as the host and target platform. The build job compiles with lint aspects, checks Rust and Starlark formatting and documentation links, and runs the dependency command as a dry run. The test job runs the optimized test suite without repeating the lint checks. Both have a 60-minute timeout. Their results appear as commit statuses on GitHub.

GitHub Actions runs the [Verify workflow](../.github/workflows/verify.yml) on every push to `main` and every pull request, including pull requests from forks: the test suite natively on each of the six [supported platforms](build.md#platforms), the browser check on ARM64 macOS, formatting, links and the dependency command. The [Book workflow](../.github/workflows/book.yml) tests and publishes the webbook, and the [Release workflow](../.github/workflows/release.yml) builds release archives when a version is tagged, as [build organization](build.md#releases) describes.

## Hosted capacity

Every step, including pipeline upload, uses the existing `linux-small` hosted queue: AMD64 Linux with 2 vCPU and 4 GB memory. Buildkite’s [Free plan](https://buildkite.com/pricing/) includes up to 2,000 Linux vCPU minutes per month on this shape. The allowance is finite; this configuration does not require paid machine shapes or self-hosted agents.

Buildkite verifies Linux x86-64 only; the Verify workflow covers the other platforms and the browser check, which needs ARM64 macOS and also runs locally with `bazel test -c opt //book:check`.

The test job selects Bazel’s `continuous` configuration, which excludes tests tagged `memory` and runs at most one test action at a time. Compilation retains two jobs. This reserves headroom for Bazel and build actions on the 4 GB agent while keeping ordinary semantic, native/WebAssembly and allocation checks enabled.

The following fixtures exceed 2 GiB of resident memory individually and carry the `memory` tag:

| Test | Peak resident memory |
| --- | ---: |
| `//program/ternary/case:repeated` | 6.88 GB |
| `//program/ternary:performance` | 3.29 GB |
| `//program/ternary/case:width` | 2.87 GB |
| `//program/ternary:infix.check` | 2.64 GB |
| `//theorem/coloring:progression.nine.order` | 3.36 GB |

These are native ARM64 macOS measurements of revision `91fbf9c`, collected with `bazel test -c opt //... --run_under='/usr/bin/time -l' --jobs=1 --test_output=all`; `//theorem/coloring:progression.nine.order` was measured the same way when it was added. They establish resource classification, not Linux memory equivalence. Every untagged test measured below 2 GiB; the largest was `//program/ternary:expression.check` at 1.92 GB. Run the full suite locally with `bazel test -c opt //...`; ordinary runs include all five large fixtures, so a machine with less memory should add `--config=continuous`. Hosted CI verifies the selected suite, and the full suite remains a release gate.

## Pipeline configuration

The pipeline editor contains [bootstrap.yml](../.buildkite/bootstrap.yml). It publishes the aggregate **Verification** status from the start of each build and uploads [pipeline.yml](../.buildkite/pipeline.yml) from the checked-out commit. Each verification step publishes its own explicitly named GitHub status. New commits supersede older queued and running builds on the same branch.

Repository hooks install checksum-verified Bazelisk 1.28.1, which reads the Bazel version from `.bazelversion`. Bazel supplies the compiler and every build dependency. The generated, ignored `user.bazelrc` limits Bazel to two jobs and a 1 GB server heap for the small agent. Post-command hooks upload test logs and XML reports as build artifacts and shut down Bazel even when verification fails.

Bazel and Bazelisk caches live outside the checkout under `/tmp/photonic`. They are local to the ephemeral agent and are discarded with it. Persistent cache volumes are not included in the Free plan and are not requested. Builds therefore work from an empty cache without any external cache service.

## Webbook

The [Book workflow](../.github/workflows/book.yml) runs on every push to `main`. It tests the record, the route, the book's scripts and the WebAssembly engine, builds `//book:site` with the `ci` configuration and publishes it with GitHub Pages; only its deploy job may write Pages, and a later push waits for a running deployment instead of cancelling it. It installs the same checksum-verified Bazelisk as the Buildkite hooks. Its cache holds Bazel's action and repository caches, keyed by `.bazelversion`, `MODULE.bazel.lock` and `Cargo.lock`, so it is saved once per dependency set instead of growing with every commit. Every GitHub workflow pins its actions by commit and keeps no credentials in its checkouts.

## GitHub integration

The Buildkite GitHub App is connected to Vantle. The repository webhook sends `push`, `pull_request`, and `merge_group` events to the pipeline’s Buildkite-generated URL with JSON encoding and TLS verification. Keep that URL in service configuration rather than committing it. Receiving merge-group events does not itself enable Buildkite merge-queue builds.

Branch and pull-request builds are enabled. Require **Verification** from the Buildkite GitHub App on `main`, with the branch up to date before merging. It covers pipeline upload and both verification jobs. Disable **Update commit statuses** in the pipeline’s GitHub settings to suppress Buildkite’s autogenerated name; explicit notifications continue to publish the aggregate and per-job statuses. Third-party fork builds remain disabled in Buildkite to avoid spending the organization’s limited hosted allowance on untrusted submissions; the Verify workflow checks them instead, so a pull request from a fork needs its checks, not Buildkite’s **Verification**, to merge.

The repository is public and clones over HTTPS without a GitHub checkout credential. No API token or self-hosted agent token is required in the repository. Manual verification is available through **New Build** in Buildkite.

Validate pipeline syntax locally with:

```sh
BUILDKITE_AGENT_ACCESS_TOKEN=validation buildkite-agent pipeline upload --dry-run .buildkite/bootstrap.yml
BUILDKITE_AGENT_ACCESS_TOKEN=validation buildkite-agent pipeline upload --dry-run .buildkite/pipeline.yml
```
