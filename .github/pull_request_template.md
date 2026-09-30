Say what the change does and why, as the commit message would.

- [ ] `bazel test -c opt //...` passes.
- [ ] `bazel run -c opt //:format -- --check` and `bazel run -c opt //:link` pass.
- [ ] The documents that describe what changed are in step, and `book/record.js` is regenerated if the webbook's examples changed.
- [ ] A change to an interface that [the compatibility contract](https://github.com/Vantle/photonic/blob/main/document/compatibility.md) covers has an entry in [CHANGELOG.md](https://github.com/Vantle/photonic/blob/main/CHANGELOG.md).
