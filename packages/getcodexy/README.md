# Codexy

Codexy is a Codex harness for coordinating agents through planning,
implementation, verification, and review.

It helps Codex turn a broad repository request into an owned plan, divide work
among agents, implement changes, verify actual outcomes, and keep the next
handoff tied to current evidence.

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
```

Start a new shell so the updated tool path is available, then install Codexy:

```sh
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
remove an optional component:

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
