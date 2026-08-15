# Public extension contracts

Installed extensions invoke these contracts by asking the host-discovered `$orchestration` skill by
name. They MUST NOT derive a core filesystem path or run a private core script.

| Contract               | Required effect                                                                                                                                      |
| ---------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| `issue-intake receipt` | Validate the canonical receipt, require parent approval before an issue mutation, and retain unsupported observations as handoff-only.               |
| `child-lane-ownership` | Validate that the named child owns its issue-sized branch/worktree lane before accepting implementation evidence.                                    |
| `completion-handoff`   | Require captured PR state, review-thread evidence when applicable, and the repository's public completion-handoff validator before a terminal claim. |

The extension supplies its domain-specific captured data; `$orchestration` applies these public
coordination rules in the active host task.
