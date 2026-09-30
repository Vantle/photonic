# Security

## Reporting a vulnerability

Report vulnerabilities privately through GitHub: open the [Security tab](https://github.com/Vantle/photonic/security) of the repository and choose "Report a vulnerability". Please do not open a public issue for a vulnerability. Include the program, the command and the version (`photonic --version`) that show the problem.

Fixes land in the latest release. Older releases are not patched.

## Trust model

A Photonic program only rewrites configurations. The language has no way to read or write files, open connections or start processes, so running a program cannot reach outside the search.

A search is bounded by its budgets: work, configurations, coherences, occurrences, scopes and records. The defaults keep small programs small, but a large program can still use several gigabytes of memory and all of the machine's cores within them. Run programs you do not trust with smaller budgets and with operating-system limits on memory and time.

The `photonic` command reads the files it is named, with the permissions of the user who runs it. `photonic mcp` does the same for every path its client sends, so connect it only to clients you trust with those files.

`bazel run //book:serve` listens on 127.0.0.1:8080 only and serves the webbook's listed files.
