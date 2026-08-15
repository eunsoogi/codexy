# Specialist role equivalence

Codexy retains seven distinct implementation specialist roles. This mapping preserves former capabilities without retaining removed callable aliases.

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
| `codexy-weaver` | Move | GitHub/integration contract in the Codexy GitHub package. |

`codexy-inspector` is reserved for #562 as the distinct bounded standard-review role. It is packaged separately from the seven retained implementation specialists and is not an alias for Auditor or Sentinel.
