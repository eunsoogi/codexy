"""Bind message recipients to the reconciled supervision registry."""

from contextlib import closing
import sqlite3

from .supervision_reservations import ReservationError
from .recipient_initialization import open_recipient_store
from .thread_delivery import RecipientAssignment, resolve_known_pair


def resolve(plugin_data: str, sender: str, recipient: str) -> RecipientAssignment:
    """Read both relationships and registered identities in one snapshot."""
    unavailable = RecipientAssignment("unavailable")
    try:
        store = open_recipient_store(plugin_data)
        with closing(store._connect()) as db:
            db.execute("BEGIN")
            rows = db.execute(
                "SELECT * FROM reservations WHERE (parent=? AND child=?) OR (parent=? AND child=?)",
                (sender, recipient, recipient, sender),
            ).fetchall()
            registered = db.execute(
                "SELECT parent FROM supervision_parents WHERE parent IN (?,?) "
                "UNION SELECT parent FROM reservations WHERE parent IN (?,?) "
                "UNION SELECT child FROM reservations WHERE child IN (?,?)",
                (sender, recipient, sender, recipient, sender, recipient),
            ).fetchall()
            bootstrapped = {
                row["parent"]
                for row in db.execute(
                    "SELECT parent FROM supervision_parents WHERE parent IN (?,?)",
                    (sender, recipient),
                ).fetchall()
            }
        # Supervision must bootstrap before it resumes; registered identities with
        # no matching relationship are unknown, including released reservations.
        # An initialized, readable registry with neither identity registered keeps
        # ordinary messaging on the existing validation path.
        if not rows:
            return RecipientAssignment(
                "unrelated" if sender != recipient and not registered else "unavailable"
            )
        # A retained owner is authoritative only after its parent reconciles the
        # complete roster, even when an isolated active row already exists.
        if any(row["parent"] not in bootstrapped for row in rows):
            return unavailable
        records = {(row["parent"], row["child"]): dict(row) for row in rows}
        return resolve_known_pair(sender, recipient, lambda p, c: records.get((p, c)))
    except (OSError, ValueError, sqlite3.Error, ReservationError):
        return unavailable
