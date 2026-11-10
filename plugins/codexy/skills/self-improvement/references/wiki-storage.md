# Wiki storage and continuation

Wiki is derived context, not task authority. Recovery MUST NOT write memory.
Select only relevant evidence, and verify live owner, scope and stop conditions.

## Storage

- Resolve paths without writes using `python3 scripts/wiki_paths.py <directory>`
  from this skill directory. `CODEXY_DATA_HOME`, when set to an absolute path,
  selects its `wiki/` child for the global Wiki. Otherwise macOS uses
  `~/Library/Application Support/Codexy/wiki`, Linux uses
  `$XDG_DATA_HOME/codexy/wiki` or `~/.local/share/codexy/wiki`, and Windows uses
  `%LOCALAPPDATA%/Codexy/wiki`. An unsupported platform requires the override.
- Repository Wiki is `<canonical Git common directory>/codexy/wiki`, outside
  disposable worktrees; task archives are its sibling `codexy/task-archives/`.
  An absolute `CODEXY_REPOSITORY_DATA_HOME` selects an alternate parent, under
  which a hash of the canonical common directory partitions clones. Linked
  worktrees share both scopes; separate clones do not.
- Each issue worktree has a separate untracked `.wiki/` for briefs, evidence,
  and candidates. For non-Git work, `--standalone` with an absolute existing
  task directory uses that directory as identity and its `.wiki/` as local
  scope; it provides no repository scope or automatic exclusion.
- MUST validate destinations and permissions, resolve Git's local exclude file,
  and verify exclusion. MUST NOT overwrite, untrack, move, or delete tracked,
  unrelated, pre-existing, or unsafe paths. Use additive migration or an
  alternate destination. Exclusion is not encryption or backup protection.

Path resolution alone neither initializes nor authorizes a write. At an
authorized issue-worktree preparation boundary, initialize the local Wiki only
after the destination and exclusion checks pass; load or reference only relevant
shared material, with source identity and revision. For non-Git tasks, a local
scope requires an authorized explicit destination and no Git exclusion claim.
`python3 scripts/wiki_init.py <worktree-directory>` initializes a new local Wiki
and adds an exclusion to Git's resolved local exclude file if needed. It refuses
pre-existing or tracked `.wiki` content and never repairs it by removal. For an
explicitly authorized non-Git task, pass `--standalone` and an absolute task
directory; this mode refuses Git worktrees and makes no exclusion claim. If
initialization fails, MUST preserve any partial files and report the failure; an
existing Wiki needs explicit additive migration or an alternate destination. For
a new repository Wiki, `python3 scripts/wiki_shared_init.py <worktree>` creates
the shared Wiki and separate archive only in a previously absent, authorized
location. An alternate parent MUST exist outside Git worktrees. Existing
destinations require explicit migration or another configured parent. Global
destinations likewise MUST be outside tracked Git content or already verified
ignored and untracked. When explicitly authorized,
`python3 scripts/wiki_global_init.py` creates a new global Wiki at the resolved
location; existing data and unsafe destinations are never replaced. Personal
installation changes remain outside this operation.

Use the [minimal contract](wiki-minimal-contract.md) for provenance, immutable
source revisions, bounded retrieval, freshness and logs, and
[migration](wiki-migration.md) for additive migration and rollback. MUST NOT
copy private material into broader scopes merely for convenience. For serialized
shared commits, selective reads, and additive revision setup, use
[Wiki revisions](wiki-transactions.md).

## Continuation

An authorized checkpoint is a derived local view, not a task database. Its
configurable budget MUST preserve the original request and corrections, owner,
scope, permissions, stop conditions, verified progress, changed artifacts and
revisions, validation results, failed approaches, remaining conditions, next
permitted action, and source identities and ranges. Missing required intent,
corrections or unresolved conditions fails validation. Preserve the last valid
checkpoint on failure and avoid accumulating redundant narration.

For a machine-checkable JSON brief use the fields `original_request`, `scope`,
`owner`, `permissions`, `stop_conditions`, `next_action`, `corrections`,
`verified_progress`, `artifacts`, `validation_results`, `failed_approaches`,
`remaining_conditions`, and `sources`. The first six are nonempty strings; the
others are arrays of nonempty strings, with at least one source. Empty arrays
explicitly mean none were found; omission is invalid. Validate using
`python3 scripts/validate_continuation_brief.py <brief>`; its default budget is
16,384 UTF-8 bytes and `--budget-bytes` configures it per task. This structural
check does not prove that an array omitted no correction or unresolved
condition: MUST compare those claims and source references against authorized
live records. MUST NOT replace the previous valid checkpoint when either check
fails. When updating a checkpoint, pass `--previous <prior-brief>`: the
validator rejects a changed original request or missing prior correction. A
removed unresolved condition or retired failure needs an entry in
`resolved_conditions` or `retired_failures`, respectively, with `item` and a
`source` also present in `sources`. A cited resolution remains a claim that MUST
be verified against the authorized source; structural validation does not prove
that resolution. The new budget applies to the replacement. The previous
checkpoint is read within its own existing byte size so a lower budget can
actually compress it.

Search an index, read bounded context, and verify sources before use. Report
unavailable evidence rather than guessing. Verify capsule integrity and live
authority. Checkpointing and actual host compaction are separate; use supported
host continuation only when available and otherwise provide a brief without
claiming a reset. MUST verify uncertain state-changing outcomes before retrying.

For large initial inputs, a source manifest MUST identify each authorized
source, revision, available size or ranges, and a narrow retrieval route rather
than injecting the entire input. MUST integrate documents incrementally and
check disjoint evidence and later corrections before deciding. A truncated tool
result MUST retain a source reference and retrieval instruction. Excluding a
source from active context MUST NOT delete it or expand permission to read it.

## Learning and retirement

Experience produces observations and scoped candidates; storage is not adoption.
Follow [rule learning](rule-learning.md), preserving evidence and
counterevidence. Promote validated repository lessons to shared Wiki and only
generally applicable, privacy-checked lessons with destination authority to
global Wiki. Memory writes do not change instructions, policies or goals.

Serialize shared promotions with revision checks and atomic replacement.
Preserve conflicting candidates and previous state on failure; advance
checkpoints only after validation and commit, and make retries idempotent.
Before authorized worktree removal, preserve needed evidence or durable source
references in a separate task archive and verify preservation. Report failures
as cleanup blockers. Archival is not promotion and MUST NOT reactivate resolved
work.

For selected local files, the optional `python3 scripts/wiki_archive.py` helper
accepts an absolute source Wiki, an existing authorized archive directory,
explicit repeated `--file` selections and an `--origin` naming the task, branch,
and revision. It publishes a hash-addressed manifest only after copying and
rechecking selected bytes. MUST place archives in the resolved separate task
archive, check destination permissions and source access before invoking it, and
inspect the returned manifest and referenced files before cleanup. The helper
never deletes the worktree. A failed or conflicting archive MUST block that
cleanup; it does not authorize copying unselected data or adopting its contents
as knowledge. Use `python3 scripts/wiki_archive.py <absolute-manifest> --verify`
for a read-only integrity check even after the original worktree has been
retired. For revision-managed material, MUST select the current pointer, its
referenced revision state and every file needed for continuation or audit; MUST
verify the archive contains those selections before cleanup.
