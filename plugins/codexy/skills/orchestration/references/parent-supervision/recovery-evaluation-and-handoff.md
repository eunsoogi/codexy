# Recovery, evaluation, and handoff

- Before cleanup, the Orchestrator MUST verify native goal completion, accept
  the result, and confirm no pending PR, CI, review, fixup, follow-up, or
  delivery work. Idle, blocked, a terminal report, or a clean worktree is
  insufficient. MUST retain the task, worktree, and branch through merge
  readback (#289). MUST follow
  [cleanup procedure](../../../../../codexy-github/skills/git-workflow/references/local-git-and-branches.md#accepted-child-cleanup):
  parent-owned preservation, direct thread archive and identity readback, then
  safe local branch removal; worktree archival is not a prerequisite.
- Missing output, truncated extraction, or a wrong item-type filter does not
  prove that host evidence is absent. Codex MUST check the actual saved
  artifact, worktree, source record, and creating surface. Codex MUST NOT invent
  a checkpoint path, retry a known-absent path, or restart completed proof after
  compaction without a concrete reason.
- When evaluation of this guidance is explicitly in scope, Codex MUST use the
  existing bounded private skill-evaluation workflow. The evaluator MUST keep
  cases, holdout inputs, expected behavior, exact invocations, raw measurements,
  and complete results in evaluator-controlled private artifacts and MUST
  publish only the allowed hash/status/failure-dimension/cost summary. The
  ordinary supervision loop MUST NOT require a private campaign, answer table,
  fixed case matrix, or unmeasured savings claim.
- When evaluation is in scope, the evaluator MUST use actual Orchestrator and
  Worker calls with the configured model/effort where those roles matter and
  MUST report unavailable telemetry as unmeasured. The evaluator MUST choose
  cases and comparisons proportionate to the changed boundary rather than
  copying a hidden answer. Observed cumulative usage totals MUST NOT be
  presented as priced, cached, or proven savings without the corresponding
  measurement.
- Codex MUST finish with current branch, PR state, exact head/base, changed
  files, actual app observations, evaluation limitations, measured versus
  unmeasured usage, retained review history, unresolved findings, and one
  Orchestrator-owned next action. A passing test, callback, open PR, or merged
  change is not a substitute for the other required evidence.
