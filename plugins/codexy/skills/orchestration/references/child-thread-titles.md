## Child Thread Titles

- After a forked Codex worktree child thread finishes setup and a thread id is
  available, the orchestrator MUST rename it with `set_thread_title` when that
  tool is available.
- The child thread title MUST clearly include the project, issue number, and
  lane purpose, such as `<project> #<issue> implementation lane`.
- If title renaming is unavailable, mention that limitation in orchestration
  status or child handoff and continue the lane.
- Child thread title renaming is a clarity policy, not a merge blocker for
  otherwise complete implementation work.
