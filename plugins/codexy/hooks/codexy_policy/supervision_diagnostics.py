"""Closed caller-visible reservation outcomes; never echo store paths or input."""

_REASONS = {
    "store_unavailable": (
        "CODEXY_SUPERVISION_STORE_UNAVAILABLE",
        "Verify the host-configured private reservation store is accessible; "
        "do not dispatch until it can be verified.",
    ),
    "parent_uninitialized": (
        "CODEXY_SUPERVISION_PARENT_UNINITIALIZED",
        "Reconcile this parent's complete existing owner roster and run "
        "bootstrap --confirmed-complete before preparing a request.",
    ),
    "unmatched_request": (
        "CODEXY_SUPERVISION_CAPACITY_UNPREPARED",
        "Reconcile any existing call or owner, then prepare the exact current "
        "creation input for this parent before dispatch.",
    ),
    "slot_limit": (
        "CODEXY_SUPERVISION_SLOT_LIMIT",
        "Keep this item queued; continue independent parent work and prepare "
        "again only after a confirmed slot release.",
    ),
    "prepare_rejected": (
        "CODEXY_SUPERVISION_PREPARE_REJECTED",
        "Check the exact creation input, route and retained issue owner; "
        "reuse an existing owner instead of creating a duplicate.",
    ),
}


def diagnostic(outcome: str) -> dict[str, str]:
    reason, action = _REASONS[outcome]
    return {"reason": reason, "nextAction": action}
