# eunsoogi-codexy

This package is the version-pinned `uvx` runtime bootstrap used by the Codexy
plugin's MCP entrypoints. It is released by Codexy's trusted publishing
workflow and is not a general-purpose command-line interface.

Cargo single-file packages still require nightly `-Zscript`, so production
startup does not use Rust script mode. See the official
[Cargo unstable feature reference](https://doc.rust-lang.org/cargo/reference/unstable.html#script).

The distribution exports one console entrypoint with two server modes:

- `codexy-mcp-runtime {lsp,codegraph} --plugin-root PATH -- ...`
