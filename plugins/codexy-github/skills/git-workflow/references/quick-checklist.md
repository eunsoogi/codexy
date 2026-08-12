# Git workflow readiness checklist

- Issue exists or a maintainer provided an explicit issue-sized scope.
- `$orchestration` classified the lane and recorded type, owner, scope, skills,
  tools/evidence, and first allowed action.
- Branch is not the configured default branch, follows active policy, and lives
  in an isolated worktree.
- No unrelated files are staged; no force push or force-with-lease is used.
- Issue and PR titles have been validated before their mutations.
- Verification covers touched surfaces, including public touched-LOC validation.
- Code changes include Codegraph findings and LSP or fallback evidence.
- Non-trivial work has the `$orchestration` review-profile-selected review.
- PR body has structured sections and one final `Fixes #<issue-number>` line.
- No actionable review feedback or review threads remain.
- Squash merge bodies preserve the PR body; branch deletion and main sync are proven.

A checked contract is the sole merge authorization; generic completion signals
and a ready PR are non-authoritative.
