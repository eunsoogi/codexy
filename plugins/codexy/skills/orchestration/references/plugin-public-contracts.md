# Public extension contracts

- `issue-intake receipt`: validate the canonical receipt and require parent
  approval before issue mutation; unsupported observations remain handoff-only.
- `child-lane-ownership`: verify the named child owns its issue-sized
  branch/worktree lane before accepting implementation evidence.
- `completion-handoff`: require current PR, review-thread, head, local/remote,
  label, and active-project handoff evidence before a terminal claim.

GitHub or Devtools work MUST invoke its installed public extension; an
unavailable required extension MUST fail closed. Extensions MUST invoke only
the applicable named contract and MUST NOT derive private core paths or
substitute missing proof.
