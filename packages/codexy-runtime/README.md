# Codexy runtime

`codexy-runtime` is the Rust package that provides Codexy's runtime binaries,
including the MCP LSP and Codegraph servers, the plugin validator, and release
support tools.

This package is owned by the `packages/codexy-runtime` module root. For the
plugin, repository workflows, and user-facing setup, see the
[repository guide](../../README.md).

## Local development

Run the Rust suite from the repository root with an explicit manifest:

```sh
cargo test --manifest-path packages/codexy-runtime/Cargo.toml --locked --all-targets
```
