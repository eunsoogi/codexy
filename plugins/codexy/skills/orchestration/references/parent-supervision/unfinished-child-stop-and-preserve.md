# Unfinished child stop-and-preserve

On an explicitly authorized stop-and-preserve request, the Orchestrator MUST
stop new issue assignments and coordinate cessation of active writes with each
current Worker. A cooperative freeze message, delivery receipt, or idle-looking
task MUST NOT be treated as proof that an in-flight write stopped. The
Orchestrator MUST observe the exact task and host with the latest `wait_threads`
cursor and inspect the relevant result, diff, or state. If the host cannot
hard-stop an in-flight operation, it MUST report that host boundary and keep
that child's archival pending. It MAY continue independent authorized
preservation and archival for children whose work is already recoverable.

After write cessation is observed, for each unfinished child, the Orchestrator
MUST verify an inventory from the owning worktree: issue and owner, task and
worktree identity, branch and exact head, staged and unstaged changes, untracked
files, required ignored data, remaining obligations and proof, and a concrete
resume location. It MUST preserve the work within the request's explicit
authority and retain exactly one continuation owner for the issue. A clean
status, delivered message, open Draft PR, or claimed tool cancellation alone
MUST NOT prove that the work is recoverable. The
[local Git preservation guidance](../../../../../codexy-github/skills/git-workflow/references/local-git-and-branches.md#unfinished-work-preservation)
defines the inventory, sensitive-data, publication, and deletion boundaries.

Before any separately authorized archival, the Orchestrator MUST read back the
actual preserved work, remaining obligations, resume location, and continuation
owner. If preservation fails or remains unknown, it MUST defer archival only for
that child, retain its branch, worktree, and required ignored data, and continue
independent authorized work. A generic stop request or usage limit MUST NOT
authorize remote publication, archival, or deletion. The Orchestrator MUST ask
only for an actual unresolved preservation or disposal decision; an unavailable
host operation MUST be reported as a host boundary, not missing user authority.
It MUST NOT mark a child goal or issue complete because work stopped, was
preserved, or was archived. Before task or worktree archival, it MUST verify
that the authorized preserved location will remain accessible afterward; if host
behavior is unknown, defer that child's archival and report the boundary.
Accepted-child cleanup in
[parent supervision](../parent-supervision.md#recovery-evaluation-and-handoff)
applies only after acceptance and MUST NOT discard or replace this
unfinished-work flow.
