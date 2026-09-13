# AGENTS.md

Use one-word identifiers, never abbreviating. Always avoid plural identifiers. Prefer namespaces over identifier mashing. Use familiar human names; let the structure supply context.

Avoid duplicate identifiers. Refactor instead of repeating a word to describe a namespace.

Prefer files without namespace nesting. Split responsibilities into multiple files.

Never comment. If a comment is needed, explain why.

Prefer immutability.

Avoid aliases and complex indirection.

Prefer unindented, single-purpose, reusable components.

Prefer explicit code with inferred types. Do not duplicate information in annotations.

Prefer accurate, precise, composable type abstractions over untyped data.

Prefer structured, organized result error types.

Prefer early returns.

Separate concerns. Compose behavior; do not pollute layers.

Prefer clean breaks over legacy compatibility.

Test code when an interface commitment is stable.

Unless stated otherwise, delete dead code.

Ensure multiplatform hermetic builds with only Bazel as the system dependency.

Prefer explicit build and configuration instructions. Never use globs in build rules. Group related files explicitly by responsibility. Keep package visibility private and grant only the access each consumer needs.

Fantastic software is built with robust, composable layers.

## Additional

Repository skills use the open Agent Skills format under `.agents/skills/$SKILL/SKILL.md`.
