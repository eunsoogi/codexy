# Specialist role equivalence

Codexy packages seven distinct specialist roles. This mapping preserves the
former capabilities without retaining removed callable aliases.

| Former role | Disposition | Capability owner |
| --- | --- | --- |
| `codexy-architect` | Retain | Architecture and durable schema boundaries. |
| `codexy-auditor` | Retain | Acceptance evidence and observable QA. |
| `codexy-cartographer` | Retain | Read-only repository and ownership mapping. |
| `codexy-forge` | Remove | The generic owning child performs scoped implementation. |
| `codexy-pathfinder` | Remove | Orchestration owns classification, planning, and approach selection. |
| `codexy-scribe` | Remove | The owning child drafts its own documentation and handoff. |
| `codexy-sculptor` | Remove | The engineering workflow owns behavior-preserving refactoring. |
| `codexy-sentinel` | Retain | Independent strict review; fixed at `gpt-5.6-sol` / `xhigh`. |
| `codexy-shipwright` | Retain | Release, package, and rollback readiness. |
| `codexy-tracer` | Remove | The engineering workflow owns diagnosis and regression investigation. |
| `codexy-warden` | Retain | Security, permission, shell, and state-mutation boundaries. |
| `codexy-weaver` | Retain | GitHub/integration contract; its future physical move belongs to the GitHub-plugin work. |

`codexy-inspector` is reserved for #562 as a future standard-review role. It
is not packaged, catalogued, or routed by this reduction, and it is not an alias
for Auditor or Sentinel.
