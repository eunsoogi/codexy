# codexy-runtime-tools

This package is the version-pinned `uvx` runtime bootstrap used by the Codexy
plugin's MCP entrypoints. It is released by Codexy's trusted publishing
workflow and is not a general-purpose command-line interface.

Cargo single-file packages still require nightly `-Zscript`, so production
startup does not use Rust script mode. See the official
[Cargo unstable feature reference](https://doc.rust-lang.org/cargo/reference/unstable.html#script).

The distribution exports two console entrypoints:

- `codexy-mcp-runtime {lsp,codegraph} --plugin-root PATH -- ...`

Downstream update automation can import the quiet API directly:

```python
from codexy_runtime_tools.updater import SyncResult, sync_agents

result: SyncResult = sync_agents(plugin_root, codex_home, "check")
```

`sync_agents` accepts `check`, `install`, `uninstall`, and `diagnose`; it
returns a structured `SyncResult` and never writes unsolicited output.
