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

This locates the preservation responsibility at the native
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

## Bounded native preservation result

A fresh parent-prepared source consumed candidate instructions at
`eb90f766f83badd18f63f3fdb90fa80f5ff3e258`. Its clean disposable checkout
remained detached at `81f4cc2bbe5c9efee342dbf523c04091caf0ef56`; this was
instruction verification, not execution of a newly installed host repair.

- Source `01a1818a-bd77-7de2-97bf-fd7e145cb7f6` self-attached using the
  supported native tool and genuinely completed its preparation objective.
  Source artifact: `01a1818f-0a1c-7312-8d8d-bf1f79356d89`.
- One same-directory fork, target `01a18191-f2c2-7250-b83d-0837df008321`,
  inherited distinct artifact `01a18191-f35f-71b0-96c3-c70643dfe745` with the
  same native checkout identity. Target self-attachment succeeded with
  `registrationError=null`; pre-archive process/identity/file-state checks
  passed.
- After separate authorization and the source terminal receipt, the parent
  archived only the source chat once. The target's actual archived/active
  inventories confirmed the source archived and exactly one active target owner.
  No worktree archival, deletion or cleanup was performed.
- Target turn `01a1819b-1c75-7673-89e0-a4724c1e34f8` records fresh native null →
  exact verification objective → active before artifact/process reads. First
  process `exec-45a988e3-b889-4cb6-b7d5-e2fd1ff2f39c`, explicitly at the
  intended CWD, completed with exit 0. CWD/top-level, absolute Git directory,
  common directory, detached HEAD, clean index and tracked/untracked state
  matched the pre-archive baseline. The same target artifact remained present;
  no post-archive reattachment or restoration occurred.
- Original requested source settings Luna/max were preserved in target delivery;
  effective settings remained unexposed. Parent received event
  `post-archive-target-1434-01a18191-f2c2-7250-b83d-0837df008321-81f4cc2-v1`.

The
[demonstrated path](executable-checkout-recovery.md#demonstrated-completed-source-preservation-path)
is supported for these observed prerequisites. There is no longer evidence for a
blanket claim that native checkout preservation is unavailable. It remains
incorrect to infer the same guarantee for an unattached source or every fork.

## Original issue criteria

| Criterion                                                                                 | Authoritative evidence and disposition                                                                                                                                                                                                                                                                                                          |
| ----------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Identify the metadata/execution boundary                                                  | Historical source/fork records separate native task CWD from managed association and actual process creation. The responsible host boundary is fork/worktree association and archive lifetime; the repository defect was unconditional archival without preservation/process acceptance. The precise historical internal cause remains unknown. |
| Correct the repository contract or name upstream ownership and containment                | The linked recovery reference, entry prompt and completion rules now gate unsafe archival and require process/identity acceptance. The explicit self-attachment/inherited-target path above supplies a supported bounded route. Native implementation remains owned by Codex desktop.                                                           |
| Authorized native source archival, one continuation, same checkout and successful process | The fresh completed-source scenario above supplies actual task, attachment, inventory and first post-archive process evidence. This meets the named checkout/source-archive/one-continuation/process criterion without a synthetic blocked goal.                                                                                                |
| Preserve failures and distinguish containment from host repair                            | Historical source/fork goals and failure history were retained; this implementation did not clear or revive them. The new result records a supported attachment/lifetime path, not a signed-host change or an explanation of historical deletion.                                                                                               |

Generic blocked-goal recovery, arbitrary dirty/ignored work preservation, every
attachment topology and effective model preservation remain untested. They MUST
NOT be advertised as proved or added as unspecified completion requirements for
this bounded issue. Current-head repository checks, independent review and CI
remain separate PR readiness requirements.
