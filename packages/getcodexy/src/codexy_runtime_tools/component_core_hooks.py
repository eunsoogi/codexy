"""Canonical installed Codexy core hook registration paths."""

_TOOL_PREFIX = "^(?:codex_app__|mcp__codex_app__)"

COMMAND_HOOKS = (
    (_TOOL_PREFIX + "send_message_to_thread$", "codexy-thread-delivery"),
    (_TOOL_PREFIX + "create_thread$", "codexy-child-thread-creation"),
    (r"^(?:(?:agents|multi_agent_v1)__)?spawn_agent$", "codexy-subagent-ownership"),
)
LAUNCHERS = tuple(
    f"hooks/{stem}.{extension}"
    for _, stem in COMMAND_HOOKS
    for extension in ("sh", "cmd")
)
DEPENDENCIES = (
    "hooks/codexy-hook-runtime.sh",
    "hooks/codexy-thread-delivery.py",
    "hooks/codexy_policy/thread_delivery.py",
    "hooks/codexy-child-thread-creation.py",
    "hooks/codexy_policy/child_thread_creation.py",
    "hooks/codexy-subagent-ownership.py",
    "hooks/codexy_policy/subagent_ownership.py",
    "hooks/codexy_policy/envelope.py",
    "hooks/codexy_policy/thread_delivery_support.py",
)
