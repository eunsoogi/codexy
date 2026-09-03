# Codexy

Codexy is a component-aware Codex harness for owned work, specialist help, and
proof-driven completion.

It gives Codex a disciplined path from a broad repository request to an owned
implementation, observable verification, bounded review, and a safe finish.
Codexy keeps the scope, owner, current evidence, and next action visible while
work moves through planning, implementation, verification, review, and handoff.

## Components

Codexy is delivered as three cooperating components. `github` and `devtools`
each depend on `core`; the installer adds those dependencies automatically.

| Component  | Plugin            | What it adds                                                                                                                 |
| ---------- | ----------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| `core`     | `codexy`          | Orchestration, finite goals and plans, worktree ownership, specialist roles, instruction hooks, and proof-driven completion. |
| `github`   | `codexy-github`   | Issue-to-merge workflows for branches, pull requests, CI, review feedback, releases, and GitHub safety checks.               |
| `devtools` | `codexy-devtools` | Local Codegraph and LSP servers, configuration, wrappers, and developer-tool guidance.                                       |

The component model lets each repository install the capabilities it needs
without losing a single ownership and verification contract.

## Install

Install the `getcodexy` command, then install the complete Codexy product:

```sh
uv tool install getcodexy
uv tool update-shell
getcodexy install
```

The default selection installs `core`, `github`, and `devtools`. To choose a
smaller dependency closure, name the optional components you need:

```sh
getcodexy install core
getcodexy install github
getcodexy install devtools
getcodexy install github devtools
```

Open a fresh Codex session after installation or update so the host can expose
the installed plugins, skills, agents, hooks, and MCP servers.

## Maintain and check Codexy

Keep the installer and installed component selection current, inspect health, or
remove an optional component with the same component-aware lifecycle:

```sh
uv tool upgrade getcodexy
getcodexy update
getcodexy status
getcodexy doctor
getcodexy remove github
```

`status` reports the installed inventory and its consistency. `doctor` checks
host readiness and the health of each installed component. Update and removal
operations preserve the dependency closure and reject inconsistent selections.

## Project links

- [Source repository](https://github.com/eunsoogi/codexy)
- [Issue tracker](https://github.com/eunsoogi/codexy/issues)
- [License](https://github.com/eunsoogi/codexy/blob/main/LICENSE)
