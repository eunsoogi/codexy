# Message visibility and style

The [scoped assignment authority](../child-routing-policy.md) covers necessary
task creation and assignment, follow-up, reporting, and recovery messages. Codex
MUST NOT ask for approval again per child or message, or treat it as host
permission. For Worker goals, MUST reuse an applicable explicit user request
covering the Workers while respecting the host's goal-creation authority. For
setting or capability conflicts, MUST follow the
[skill and host conflict decision](../../SKILL.md#skill-and-host-conflict-decision).

Task-to-task prompts, delegated instructions, callbacks, progress updates,
corrections, and handoffs, including their tool prompt fields, MUST be treated
as user-visible and MUST follow the shared
[plain-language user-replies contract](../plain-language-user-replies.md).
