"""Decide whether a known child has reserved capacity for a later turn."""

from __future__ import annotations

import sqlite3
from typing import Literal

from .supervision_reservations import ReservationError, Reservations


def resume_decision(
    store: Reservations, parent_session: str, child_id: str
) -> Literal["active", "released", "unknown"]:
    """Look up one established relationship without classifying unrelated chats.

    The caller authenticates both identities. A missing record cannot prove that
    the recipient is unrelated to the parent, so it is returned as unknown.
    """
    try:
        owner = store.lookup(parent_session, child_id)
    except (ReservationError, OSError, sqlite3.Error):
        return "unknown"
    if not owner:
        return "unknown"
    if owner["state"] == "active":
        return "active"
    if owner["state"] == "released":
        return "released"
    return "unknown"
