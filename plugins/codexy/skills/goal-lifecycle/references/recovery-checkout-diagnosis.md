# Same-directory checkout diagnosis: issue #1434

## Contract and ownership

The requested outcome is an executable continuation at the intended checkout,
with one execution owner and preserved unfinished goals, settings and work. The
repository correction is fail-closed instruction containment; it changes no host
implementation. Native fork, managed-worktree association, cleanup and process
creation belong to the Codex desktop host. The exact upstream engineering team
and defective internal function are not exposed by these tools.

Non-goals are a replacement allocator, signed-host patch, invented transport,
repeated forks, substitute checkouts and manufactured blocked goals. The
material risk is archiving a source while its continuation depends on the same
managed checkout without proven lifetime preservation. A valid task ID is
insufficient.

## Preserved historical evidence

The authentic archived task histories for
[release issue #1422](https://github.com/eunsoogi/codexy/issues/1422) and
[activation PR #1433](https://github.com/eunsoogi/codexy/pull/1433) identify:

| Observation               | Evidence and limit                                                                                                                                                                                                                                                                      |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Executable source         | Source task `01a1785b-430c-7541-999c-254f4bdaaefc`, turn `01a17887-bb38-7a71-8126-859ae49c4667`: explicit checkout process completed with exit 0 and clean HEAD `342b9d0a0f89e6911074cc03e4fe56b89395079c`.                                                                             |
| Native fork               | The same source turn calls `fork_thread` once with `environment.type=same-directory`. Its authenticated recovery report names fork `01a17888-0ddb-7480-a6a0-066ecb23bd3e` and synchronization complete. The visible history does not expose an independent attachment-transfer receipt. |
| Source archival           | The authenticated parent continuation says the source is archived after an archived-list readback. Both source and fork are now present in the actual archived task list. This does not locate the moment the directory became unavailable.                                             |
| Failed continuation       | Fork turn `01a17889-eed0-7240-8e5f-ba01f9bd9ee4`: first explicit-CWD process failed before creation, exit -1, `Failed to create unified exec process: No such file or directory (os error 2)`.                                                                                          |
| Metadata versus execution | The fork's subsequent authenticated report records its unchanged task CWD and an inherited PR attachment but no worktree attachment. Neither record proves filesystem availability.                                                                                                     |
| Later inventory           | The parent's later `git worktree list --porcelain` excludes the former checkout. No matched inventory immediately before and after source archival is available.                                                                                                                        |

The source and fork's blocked outcomes and failure reports remain historical.
They MUST NOT be cleared, overwritten, reactivated or relabeled as successful
proof for this issue. A separately approved replacement verifier later executed
at activation commit `0e4ad51fcd20e7e36870ef2493c27e895448b0f0`; that is
evidence of replacement execution, not repaired same-directory recovery.

## Boundary established, cause unresolved

The
[official OpenAI worktree lifecycle](https://developers.openai.com/codex/app/worktrees#worktree-cleanup)
describes managed-checkout deletion when the associated chat is archived and
notes that history can outlive the worktree. Current supported tool contracts
expose `fork_thread`, `set_thread_archived`, `attach_worktree`, and
`restore_worktree` as separate operations. The fork result does not document an
atomic managed-checkout lifetime transfer. Attachment is constrained by native
ownership; restoring an archived checkout is separate from preserving this exact
checkout and is not automatic authority to resume a blocked goal.

This locates the unsupported preservation guarantee at the native
fork/association/archive-to-execution boundary. It does not prove archival
caused the historical deletion: missing fork association, host cleanup, and an
execution environment failure cannot be distinguished from the available
before/after observations. Codexy's proven defect is that its recovery
instructions accepted owner/CWD metadata and unconditionally required source
archival without checking that guarantee or the continuation's first process.

## Supported containment and acceptance gap

The [ordered recovery reference](executable-checkout-recovery.md) now requires
actual baseline identity, supported lifetime preservation before source
archival, and post-activation process/identity acceptance. If no supported
preservation route is available, the safe result is to retain the source and
report the missing native capability before fork/archive. No host repair is
claimed.

Instruction readback, link checks and plugin validation can prove the repository
contract. Authorized read-only native task/attachment/Git/process observations
can establish the current environment and unavailable route. Neither proves host
enforcement or successful recovery. An absent attachment in an otherwise
executable fresh worktree MUST NOT be treated as proof that its directory is
missing or that every worktree is broken.

## Supplementary same-task evidence

On 2026-10-28, the authenticated #1440 task history
`01a18168-e622-79d2-a0f8-2b9a01455303`, turn
`01a18179-4b70-7f81-b32b-75fa0831c022`, records the parent's direct-human
confirmation that the original goal was restarted. The child reports native
`active` for its unchanged setup objective. Its subsequent explicit-CWD process
`exec-fe1475a5-0b43-426a-aad1-d4e2d05f49ca` completed with exit 0, unchanged
checkout path and HEAD `81f4cc2bbe5c9efee342dbf523c04091caf0ef56`. The source
task identity and checkout were retained; no fork or source archival was used.
The activation readback is an authenticated child report, not an independently
queried cross-task goal result. This supports the same-task continuation branch
when the owner confirms exact native active state; it MUST NOT satisfy the fresh
fork/archive scenario or claim an archive-lifetime repair. The attempted
`/goal resume` text was not that successful route: the parent reports it was
interpreted as a replacement objective and canceled. MUST NOT recommend
replaying it as a resume command.

## Remaining native acceptance

Issue completion still requires an explicitly authorized native scenario with
supported preservation, source archival, exactly one continuation owner,
preserved checkout identity and a successful explicit-CWD process after fresh
active-goal readback. Until those results exist, that criterion MUST remain
unverified and the pull request MUST remain draft. A completed disposable source
can exercise checkout lifetime only; blocked-goal handling and unexposed
effective model settings remain separate unverified claims.
