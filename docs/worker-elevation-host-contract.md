# Worker Elevation Host Contract

## Observation record

Observed on 2026-10-28 in a local Codex task. The host/client version is not
exposed by the callable surface. This record distinguishes current callable
tools from officially documented UI controls; it does not prove a native Worker
transition or behavior on another host.

| Callable and relevant arguments                                                                                  | Exposed behavior                                                                                      | Limit for source quiescence                                                                                                                        |
| ---------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| `get_goal({})`                                                                                                   | Reads the calling task's objective and status.                                                        | Has no target-task argument; the Orchestrator cannot directly query another task's current goal.                                                   |
| `mcp__codex_app__list_threads({limit?})`                                                                         | Lists pinned and recent task IDs, host, status, project, and CWD.                                     | Does not report branch, HEAD, dirty/index state, running processes, or queued prompts.                                                             |
| `mcp__codex_app__read_thread({threadId, hostId?, cursor?, includeOutputs?, maxOutputCharsPerItem?, turnLimit?})` | Reads recent status and turn summaries, optionally with truncated command/tool outputs.               | Not a live process/queue inventory; prior source-side goal output may be stale or truncated.                                                       |
| `mcp__codex_app__wait_threads({targets:[{threadId, hostId?, afterCursor?}], timeoutMs?})`                        | Waits for up to eight target tasks to complete or need attention.                                     | Timeout, idle, or attention is not proof that current writers stopped; it does not report or suppress continuation.                                |
| `mcp__codex_app__list_artifacts({})` / `mcp__codex_app__attach_worktree({path})`                                 | Reads this task's worktree attachments; can reattach an exact existing managed checkout without reset or setup scripts. | An attachment and returned workspace path do not prove branch, HEAD, index, file identity, or post-archive survival; verify those from the target. |
| `mcp__codex_app__read_thread_terminal({})`                                                                       | Reads the current desktop task's terminal output.                                                     | Does not read another task's terminal or establish process termination.                                                                            |
| `mcp__codex_app__send_message_to_thread({threadId, prompt, hostId?, model?, thinking?})`                         | Sends a user-visible follow-up prompt to a task.                                                      | No stop/cancel guarantee is specified; treat a freeze request as cooperative only.                                                                 |
| `exec_command` / `write_stdin({session_id, chars?, yield_time_ms?, max_output_tokens?})`                         | Starts a command and can interact with an existing session ID.                                        | An exact session exit result can prove that command ended; the schema does not establish cross-task session control or detached-process inventory. |
| `agents.list_agents` / `agents.interrupt_agent({target})`                                                        | Lists agents in the calling task's tree and interrupts an exact agent ID, returning its prior status. | A tree readback exposes agent-level status only; it does not establish termination of a child command, tool session, or detached process.          |
| `mcp__codex_app__handoff_thread({threadId, destinationHostId?, followUpPrompt?})`                                | Interrupts a running other task before moving its Git state between checkout and worktree.            | Cannot target its caller and changes the checkout location; excluded from same-worktree checkpointing.                                             |

No standalone current-turn stop, queued-continuation query, or automatic-
continuation disable callable was exposed. The current
[#1446 maintainer direction](https://github.com/eunsoogi/codexy/issues/1446)
defines quiescence as the observed absence of implementation writes and live
file-writing producers under a cooperative no-write instruction that persists
across continuation turns. Automatic reasoning and read-only turns may still
happen. The checkpoint is not a terminal task state or a durable lock, and a
receiving owner MUST recheck before proceeding. The procedure MUST NOT use goal
completion, a fabricated pause, archival, or a worktree move to simulate a stop,
and MUST NOT claim future continuation is disabled.

The callable metadata also does not expose the effective model/effort of an
existing task. The requested pair MUST come from its authenticated assignment;
the effective pair remains unknown unless the source host provides a separate
readback.

## Checkpoint evidence boundary

The installed
[checkpoint procedure](../plugins/codexy/skills/orchestration/references/worker-elevation/checkpoint.md)
contains the operating steps and limits and does not depend on this developer
note. The callable schemas establish what can be requested and read; they do not
establish that current file-writing producers terminated or that checkout state
was preserved. Those claims need an issue-scoped disposable native scenario and
actual writer and same-checkout readbacks. A scenario MUST NOT test or claim
that automatic continuation was disabled; the current maintainer direction
excludes suppression from this verification scope.

Git state MUST be read in the source's same checkout. `list_threads` CWD/project
metadata does not prove branch, HEAD, staged/unstaged changes, ignored
artifacts, or lock state. Sensitive ignored contents MUST NOT be copied into a
public task record. No source Worker or writer process was interrupted or tested
in this observation; it is callable-surface evidence only.
Automatic-continuation suppression was not tested and is excluded by the current
maintainer direction.

## Fork and elevation observations

These findings use the same observation date and task surface as the checkpoint
table. “Supported” means a request route is exposed or documented; it does not
prove authenticated assignment, effective settings, or a successful
transition. The installed Codexy plugin and desktop host/client versions were
not exposed by this task.

| Boundary | Status | Observed contract and limit |
| --- | --- | --- |
| Same-directory fork | supported; boundary MUST be observed separately | `mcp__codex_app__fork_thread({threadId, environment: {type: "same-directory"}})` is exposed. It has no turn/cursor-boundary argument and may include an interrupted active turn. Same-directory calls return a `threadId`; worktree setup may instead return a pending `clientThreadId`. Read back the final target ID, lineage, project/CWD, checkout attachment, branch, HEAD, index, and files before advancing. |
| Target checkout association | supported readback and conditional reattachment | `list_artifacts({})` reads the target task's own attachments. `attach_worktree({path})` can reattach the exact existing managed checkout and does not reset it or run setup scripts; it may update the target's workspace association. The target MUST then read back CWD, branch, HEAD, index, and files. This does not create a worktree or prove archive safety. |
| Requested target model | supported request route | The official [Codex desktop model control](https://learn.chatgpt.com/docs/models#choose-a-model) selects an available model and effort beneath the composer. The current `send_message_to_thread` schema also accepts `model: "gpt-6.1-terra"` and `thinking: "medium"`, but starts a turn. The fork call has no model or effort fields. A selected or requested pair is not an authenticated or effective-pair readback; availability varies by account, client, workspace, and rollout. |
| Original-parent binding | unsupported as a fork argument; supported as addressed messaging | `fork_thread` has no parent-binding argument. `send_message_to_thread({threadId, hostId, model, thinking, prompt})` can address a known parent task; acceptance proves transport only. An actual parent-to-target follow-up and target readback are needed to establish the bidirectional route for that transition. |
| Target goal | supported in the target task; unsupported as a cross-task query | `get_goal({})` reads only the calling task and has no target ID. The target MUST perform its own goal readback. The Orchestrator MUST supply a fresh readback of its own overall goal. |
| Capacity reservation | unsupported by the callable surface | `list_threads` exposes visible task state, not an authoritative slot reservation or lower host limit. The parent MUST reserve and count source and target using its current supervision record before forking. Archival alone does not establish explicit termination/release or absence of remaining execution, so it MUST NOT release capacity. |
| Source-only retirement | archive request supported; active-goal and descendant effects unverified | `mcp__codex_app__set_thread_archived({threadId, archived: true})` addresses a task. Whether it archives descendants is unobserved. One parent-reported completed-source case showed a target's explicitly attached checkout remained usable after source archival; it does not establish active-goal elevation safety, descendant effects, or control later continuation. MUST NOT claim source resumption is disabled; post-transfer source turns MUST remain read-only under the maintainer direction. Keep the reservation until supported explicit termination/release and absence of remaining execution are confirmed. |
| Actual active-goal transition | unobserved | No #1447 active-goal target fork, source archive, owner replacement, or first target turn was executed for this observation. Effective Terra/medium assignment, authenticated parent binding, source quiescence, and target/worktree survival after active-goal source retirement remain unverified. |

## Bounded prior checkout-lifetime observation

The parent supplied this prior #1434 result: a disposable source had its managed
checkout attached; its same-directory fork inherited a distinct attachment; the
target's own attachment and read-only preflight succeeded before source-only chat
archival; and the target's first explicit-CWD process after archival succeeded
with matching identity and content. This is evidence for that completed-source
checkout-lifetime case only. It does not establish active-goal quiescence,
Terra/medium assignment, authenticated original-parent binding, absence of
concurrent writers, or the cause of the separate historical `ENOENT`. MUST NOT
repeat an archive experiment to test automatic-continuation suppression; the
current maintainer direction excludes that question.

The parent reported that Codex app computer use was unavailable in its session,
so no model selection was performed in a target UI. MUST NOT substitute AppleScript,
raw App Server calls, or CLI task creation. The official model control is a
supported request route only; the target's authenticated assignment and
effective model remain unobserved until a real target readback exposes them.

## Transition evidence boundary

The installed
[transition procedure](../plugins/codexy/skills/orchestration/references/worker-elevation/transition.md)
uses these limits and MUST stop before ownership transfer when required
authentication or preservation evidence is unavailable. Callable schemas and
official UI documentation do not establish source-writer termination, a
bidirectional parent route, or successful post-retirement worktree reuse. Those
claims require an explicitly assigned native scenario and actual readbacks. The
current maintainer direction excludes testing automatic-continuation
suppression.
