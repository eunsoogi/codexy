"""Build actionable, cause-specific feedback for denied Worker creation."""

from __future__ import annotations

from .envelope import Diagnostic
from .thread_delivery import HARD_WORKER_ROUTE, WORKER_CREATION_ROUTES, WORKER_ROUTE


# Request-field repair and parent-checkout recovery have different owners and next actions.
def creation_denial(
    code: str,
    message: str,
    data: dict[str, object] | None = None,
) -> Diagnostic:
    correction = (
        _PARENT_CHECKOUT_GUIDANCE
        if code in {"PARENT_WORKTREE", "PARENT_CHECKOUT_UNVERIFIED"}
        else _WORKER_FIELD_CORRECTION_GUIDANCE
    )
    if data is not None and code in {"MISSING_THINKING", "UNSUPPORTED_THINKING"}:
        example = _selected_route_example(data)
        if example:
            message = f"{message} {example}"
    detail = (
        f"{message} {correction} {_CREATION_OUTCOME_GUIDANCE} "
        f"{_HOST_RESTRICTION_GUIDANCE}"
    )
    # Preserve the cause in the hook reason and provide a separate model-context value.
    return Diagnostic(
        code,
        detail,
        f"CODEXY_CHILD_THREAD_CREATION_{code}: {detail}",
    )


def missing_route_denial() -> Diagnostic:
    return creation_denial("MISSING_ROUTE_FIELDS", _MISSING_FIELDS)


def app_worktree_denial() -> Diagnostic:
    return creation_denial("APP_WORKTREE_REQUIRED", _APP_WORKTREE_REQUIRED)


def missing_field_denial(fields: list[str], data: dict[str, object]) -> Diagnostic:
    names = " and ".join(fields)
    code = "MISSING_ROUTE_FIELDS" if len(fields) > 1 else f"MISSING_{fields[0].upper()}"
    message = f"Missing {names}; {_REQUIRED_ROUTE}."
    return creation_denial(code, message, data)


def unsupported_model_denial() -> Diagnostic:
    return creation_denial("UNSUPPORTED_MODEL", _UNSUPPORTED_MODEL)


def unsupported_thinking_denial(data: dict[str, object]) -> Diagnostic:
    return creation_denial("UNSUPPORTED_THINKING", _UNSUPPORTED_THINKING, data)


def parent_worktree_denial() -> Diagnostic:
    return creation_denial("PARENT_WORKTREE", _PARENT_WORKTREE)


def parent_checkout_denial() -> Diagnostic:
    return creation_denial("PARENT_CHECKOUT_UNVERIFIED", _UNVERIFIED_PARENT)


def _selected_route_example(data: dict[str, object]) -> str | None:
    """Give a minimal pair repair only when a supported model is already present."""
    model = data.get("model")
    if model == WORKER_ROUTE[0]:
        return (
            "Minimal correction if this is the existing assignment selection: "
            "preserve model='gpt-6-luna' and set thinking='max'."
        )
    if model == HARD_WORKER_ROUTE[0]:
        return (
            "Minimal correction if this is the existing assignment selection: "
            "preserve model='gpt-6.1-terra' and set thinking='medium'."
        )
    return None


# Build denials from the creation allowlist so existing-recipient routes cannot leak into admission.
_REQUIRED_ROUTE = "Worker creation requires either " + " or ".join(
    f"model='{model}' and thinking='{thinking}'"
    for model, thinking in sorted(WORKER_CREATION_ROUTES)
)
# Input errors are repaired from the assigned route and saved project, never by weakening admission.
_WORKER_FIELD_CORRECTION_GUIDANCE = (
    "For this already-authorized Worker assignment, the Orchestrator MUST reuse any already-selected "
    "Worker pair; if none is selected, classify the task evidence under the "
    "canonical child-routing policy: ordinary work uses model='gpt-6-luna' and "
    "thinking='max'; use model='gpt-6.1-terra' and thinking='medium' only when "
    "the task evidence meets the hard-work criteria. The assignment and its "
    "evidence, plus the canonical policy, are the sources for model and thinking. "
    "Use projectId from the saved project selected for this task; a Worker "
    "creation target must use target.type='project' and "
    "target.environment.type='worktree'. Do not invent a projectId or other "
    "missing task context; MUST correct only the identified fields in the "
    "original create_thread request and continue creating and assigning the "
    "Worker for this authorized assignment. "
    "MUST NOT ask the user to select or reconfirm model, thinking, or worktree. "
    "This guidance does not create authority for a new task or override an "
    "explicit no-new-task instruction."
)
_PARENT_CHECKOUT_GUIDANCE = (
    "This is a parent-checkout constraint, not an incorrect create_thread "
    "argument. A different controlling task MUST use the supported handoff to "
    "the saved project's primary repository and verify its actual CWD inside "
    "that checkout before retrying. MUST NOT change request fields, choose a "
    "different model or worktree route, or bypass host policy."
)
_CREATION_OUTCOME_GUIDANCE = (
    "This invocation was prevented before create_thread executed; it created no "
    "child. If an earlier attempt has no authoritative result, its outcome is "
    "unknown; inspect the current Codex task, thread, and owner state before "
    "retrying to avoid a duplicate Worker."
)
_HOST_RESTRICTION_GUIDANCE = (
    "If the corrected request reaches create_thread but the host returns an "
    "error, inspect that exact result to distinguish a newly identified input "
    "error from a host permission or capability restriction. Correct another "
    "field only when the result identifies it. For a host restriction, report "
    "the exact limitation; MUST NOT try another model or worktree route, repeat "
    "an unchanged request, or bypass host policy."
)
_APP_WORKTREE_REQUIRED = (
    "Worker creation requires the Codex app-managed project worktree in the original "
    "create_thread call: target.type='project', a non-empty projectId, and "
    "target.environment.type='worktree'. MUST NOT invent a projectId; use the ID "
    "from the saved project selected for this task. "
    "MUST NOT create a local or projectless task and try to retrofit it with "
    "create_worktree, fork_thread, shell git worktree add, or another detached directory."
)
_MISSING_FIELDS = f"Missing model and thinking; {_REQUIRED_ROUTE}."
_UNSUPPORTED_MODEL = (
    f"Unsupported Worker creation model; {_REQUIRED_ROUTE}. "
    "MUST NOT substitute another model or silently fall back."
)
_UNSUPPORTED_THINKING = (
    f"Unsupported Worker creation thinking; {_REQUIRED_ROUTE}. "
    "MUST NOT substitute another reasoning setting or silently fall back."
)
_PARENT_WORKTREE = (
    "Child task creation is blocked because the actual parent CWD is in a linked "
    "Git worktree. Use a supported handoff to move the Orchestrator to the saved "
    "project's primary repository, then verify the destination CWD is inside the "
    "primary checkout before retrying. `handoff_thread` cannot move its own caller; "
    "a different controlling task MUST perform the handoff."
)
_UNVERIFIED_PARENT = (
    "Child task creation is blocked because the actual parent CWD could not be "
    "verified inside the saved project's primary Git checkout. Use a supported "
    "handoff to the primary repository, verify the destination CWD and checkout, "
    "then retry. `handoff_thread` cannot move its own caller; a different "
    "controlling task MUST perform the handoff."
)
