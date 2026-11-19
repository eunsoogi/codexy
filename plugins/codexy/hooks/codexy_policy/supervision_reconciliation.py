"""Reconcile a verified pending setup without creating another reservation."""

import sqlite3
from contextlib import closing

from .supervision_reservations import ReservationError, Reservations, _required


def resolve_pending(
    store: Reservations, parent: str, request: str, pending: str, child: str
) -> None:
    """Bind the observed pending-to-child mapping to the same reserved slot.

    The caller supplies an authenticated ready observation. A mismatched or
    missing pending identifier never changes the retained reservation.
    """
    parent, request, pending, child = map(_required, (parent, request, pending, child))
    with closing(store._connect()) as db, db:
        db.execute("BEGIN IMMEDIATE")
        row = store._row(db, parent, request)
        if row["pending"] != pending:
            raise ReservationError("pending identity does not match")
        if row["state"] == "active" and row["child"] == child:
            return
        if row["state"] != "setting_up" or row["child"] is not None:
            raise ReservationError("reservation is not awaiting this setup")
        try:
            db.execute(
                "UPDATE reservations SET state='active',child=? WHERE parent=? AND request=?",
                (child, parent, request),
            )
        except sqlite3.IntegrityError as exc:
            raise ReservationError("child already has another reservation") from exc
