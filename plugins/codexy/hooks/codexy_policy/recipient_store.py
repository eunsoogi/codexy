"""Bind message recipients to the reconciled supervision registry."""

from contextlib import closing
import sqlite3

from .supervision_reservations import ReservationError
from .supervision_storage import open_store
from .thread_delivery import RecipientAssignment, resolve_known_pair


def resolve(plugin_data: str, sender: str, recipient: str) -> RecipientAssignment:
    """Read both relationships and complete-roster evidence in one snapshot."""
    unavailable = RecipientAssignment("unavailable")
    try:
        store = open_store(plugin_data)
        with closing(store._connect()) as db:
            db.execute("BEGIN")
            rows = db.execute(
                "SELECT * FROM reservations WHERE (parent=? AND child=?) OR (parent=? AND child=?)",
                (sender, recipient, recipient, sender),
            ).fetchall()
            parents = db.execute(
                "SELECT parent FROM supervision_parents WHERE parent IN (?,?)",
                (sender, recipient),
            ).fetchall()
        # Only two independently complete rosters can exclude both directions.
        # Absence of a record by itself never establishes an unrelated message.
        if not rows:
            reconciled = {row["parent"] for row in parents}
            return RecipientAssignment(
                "unrelated"
                if sender != recipient and {sender, recipient} <= reconciled
                else "unavailable"
            )
        records = {(row["parent"], row["child"]): dict(row) for row in rows}
        return resolve_known_pair(sender, recipient, lambda p, c: records.get((p, c)))
    except (OSError, ValueError, sqlite3.Error, ReservationError):
        return unavailable
