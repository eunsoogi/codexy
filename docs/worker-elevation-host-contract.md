# Worker Elevation Host Contract

## Observation record

Observed on 2026-10-28 in a local Codex task. The host/client version is not
exposed by the callable surface. These findings describe only the callable tools
available to that task; they do not prove behavior on another host or through an
unobserved UI control.

| Callable and relevant arguments                                                                                  | Exposed behavior                                                                           | Limit for source quiescence                                                                                                                        |
| ---------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| `get_goal({})`                                                                                                   | Reads the calling task's objective and status.                                             | Has no target-task argument; the Orchestrator cannot directly query another task's current goal.                                                   |
| `mcp__codex_app__list_threads({limit?})`                                                                         | Lists pinned and recent task IDs, host, status, project, and CWD.                          | Does not report branch, HEAD, dirty/index state, running processes, or queued prompts.                                                             |
| `mcp__codex_app__read_thread({threadId, hostId?, cursor?, includeOutputs?, maxOutputCharsPerItem?, turnLimit?})` | Reads recent status and turn summaries, optionally with truncated command/tool outputs.    | Not a live process/queue inventory; prior source-side goal output may be stale or truncated.                                                       |
| `mcp__codex_app__wait_threads({targets:[{threadId, hostId?, afterCursor?}], timeoutMs?})`                        | Waits for up to eight target tasks to complete or need attention.                          | Timeout, idle, or attention is not proof that current writers stopped; it does not report or suppress continuation.                                |
| `mcp__codex_app__read_thread_terminal({})`                                                                       | Reads the current desktop task's terminal output.                                          | Does not read another task's terminal or establish process termination.                                                                            |
| `mcp__codex_app__send_message_to_thread({threadId, prompt, hostId?, model?, thinking?})`                         | Sends a user-visible follow-up prompt to a task.                                           | No stop/cancel guarantee is specified; treat a freeze request as cooperative only.                                                                 |
| `exec_command` / `write_stdin({session_id, chars?, yield_time_ms?, max_output_tokens?})`                         | Starts a command and can interact with an existing session ID.                             | An exact session exit result can prove that command ended; the schema does not establish cross-task session control or detached-process inventory. |
| `agents.list_agents` / `agents.interrupt_agent({target})`                                                        | Lists or interrupts agents in the calling task's agent tree.                               | Does not cover the source task's own turn; interrupt may wait for an in-flight tool call, so read the tree back afterward.                         |
| `mcp__codex_app__handoff_thread({threadId, destinationHostId?, followUpPrompt?})`                                | Interrupts a running other task before moving its Git state between checkout and worktree. | Cannot target its caller and changes the checkout location; excluded from same-worktree checkpointing.                                             |

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

## Evidence boundary

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
