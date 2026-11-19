"""Transactional, parent-scoped supervision reservations.

The caller supplies authenticated observations; this module does not infer task
lineage or termination from app listings. Database location and permissions are
owned by the caller.
"""

from __future__ import annotations

import sqlite3
from contextlib import closing
from pathlib import Path


class ReservationError(ValueError):
    """The requested transition conflicts with retained ownership."""


def _required(value: str) -> str:
    if not isinstance(value, str) or not value.strip() or value != value.strip():
        raise ReservationError("identifiers and route fields must be nonempty")
    return value


class Reservations:
    """A persistent registry; each mutation serializes admission and state change."""

    def __init__(self, path: Path):
        self.path = path
        with closing(self._connect()) as db, db:
            db.execute("""CREATE TABLE IF NOT EXISTS reservations (
                parent TEXT NOT NULL, request TEXT NOT NULL, issue TEXT NOT NULL,
                model TEXT NOT NULL, thinking TEXT NOT NULL,
                state TEXT NOT NULL, pending TEXT, child TEXT,
                PRIMARY KEY (parent, request), UNIQUE (parent, child))""")
            db.execute("""CREATE UNIQUE INDEX IF NOT EXISTS active_issue
                ON reservations(parent, issue)""")
            db.execute("""CREATE UNIQUE INDEX IF NOT EXISTS pending_identity
                ON reservations(parent, pending) WHERE pending IS NOT NULL""")

    def _connect(self) -> sqlite3.Connection:
        db = sqlite3.connect(self.path, timeout=10, isolation_level=None)
        db.row_factory = sqlite3.Row
        return db

    def reserve(
        self,
        parent: str,
        request: str,
        issue: str,
        model: str,
        thinking: str,
        limit: int = 8,
    ) -> str:
        """Reserve once; retrying the same request cannot consume another slot."""
        parent, request, issue, model, thinking = map(
            _required, (parent, request, issue, model, thinking)
        )
        if isinstance(limit, bool) or not isinstance(limit, int) or limit < 0:
            raise ReservationError("limit must be a nonnegative integer")
        with closing(self._connect()) as db, db:
            db.execute("BEGIN IMMEDIATE")
            row = db.execute(
                "SELECT * FROM reservations WHERE parent=? AND request=?",
                (parent, request),
            ).fetchone()
            if row:
                if (row["issue"], row["model"], row["thinking"]) != (
                    issue,
                    model,
                    thinking,
                ):
                    raise ReservationError(
                        "request identity already has different facts"
                    )
                if row["state"] != "released":
                    return "existing"
                # Resume the same owner only after capacity is reserved again.
                count = db.execute(
                    "SELECT count(*) FROM reservations WHERE parent=? AND state!='released'",
                    (parent,),
                ).fetchone()[0]
                if count >= min(8, limit):
                    return "queued"
                resumed_state = "active" if row["child"] else "reserved"
                db.execute(
                    "UPDATE reservations SET state=?, pending=NULL WHERE parent=? AND request=?",
                    (resumed_state, parent, request),
                )
                return "resumed" if row["child"] else "reserved"
            owner = db.execute(
                "SELECT 1 FROM reservations WHERE parent=? AND issue=?", (parent, issue)
            ).fetchone()
            if owner:
                raise ReservationError("issue already has an owner")
            count = db.execute(
                "SELECT count(*) FROM reservations WHERE parent=? AND state!='released'",
                (parent,),
            ).fetchone()[0]
            if count >= min(8, limit):
                return "queued"
            db.execute(
                "INSERT INTO reservations(parent,request,issue,model,thinking,state) VALUES(?,?,?,?,?,'reserved')",
                (parent, request, issue, model, thinking),
            )
            return "reserved"

    def setup(self, parent: str, request: str, pending: str) -> None:
        """Attach a pending setup identity without allocating another slot."""
        _required(pending)
        with closing(self._connect()) as db, db:
            db.execute("BEGIN IMMEDIATE")
            row = self._row(db, parent, request)
            if row["state"] not in ("reserved", "setting_up") or row["pending"] not in (
                None,
                pending,
            ):
                raise ReservationError("conflicting setup identity")
            try:
                db.execute(
                    "UPDATE reservations SET state='setting_up', pending=? WHERE parent=? AND request=?",
                    (pending, parent, request),
                )
            except sqlite3.IntegrityError as exc:
                raise ReservationError(
                    "setup already bound to another request"
                ) from exc

    def adopt(
        self,
        parent: str,
        request: str,
        issue: str,
        model: str,
        thinking: str,
        child: str,
    ) -> None:
        """Record an existing, authenticated owner even when already over capacity."""
        parent, request, issue, model, thinking, child = map(
            _required, (parent, request, issue, model, thinking, child)
        )
        with closing(self._connect()) as db, db:
            db.execute("BEGIN IMMEDIATE")
            existing = db.execute(
                "SELECT * FROM reservations WHERE parent=? AND request=?",
                (parent, request),
            ).fetchone()
            if existing:
                if (
                    existing["issue"],
                    existing["model"],
                    existing["thinking"],
                    existing["child"],
                    existing["state"],
                ) != (issue, model, thinking, child, "active"):
                    raise ReservationError(
                        "existing reservation conflicts with observation"
                    )
                return
            try:
                db.execute(
                    "INSERT INTO reservations(parent,request,issue,model,thinking,state,child) VALUES(?,?,?,?,?,'active',?)",
                    (parent, request, issue, model, thinking, child),
                )
            except sqlite3.IntegrityError as exc:
                raise ReservationError(
                    "existing owner conflicts with observation"
                ) from exc

    def ready(self, parent: str, request: str, child: str) -> None:
        """Bind an authenticated ready response to the original route and slot."""
        _required(child)
        with closing(self._connect()) as db, db:
            db.execute("BEGIN IMMEDIATE")
            row = self._row(db, parent, request)
            if row["state"] not in ("reserved", "setting_up", "active") or row[
                "child"
            ] not in (None, child):
                raise ReservationError("conflicting child identity")
            try:
                db.execute(
                    "UPDATE reservations SET state='active', child=? WHERE parent=? AND request=?",
                    (child, parent, request),
                )
            except sqlite3.IntegrityError as exc:
                raise ReservationError(
                    "child already bound to another request"
                ) from exc

    def release(self, parent: str, request: str, *, confirmed: bool) -> None:
        """Only a caller-verified terminal execution or ownerless failure frees capacity."""
        if confirmed is not True:
            raise ReservationError("release requires confirmed evidence")
        with closing(self._connect()) as db, db:
            db.execute("BEGIN IMMEDIATE")
            self._row(db, parent, request)
            db.execute(
                "UPDATE reservations SET state='released' WHERE parent=? AND request=?",
                (parent, request),
            )

    def lookup(self, parent: str, child: str) -> dict[str, object] | None:
        """Return the retained assignment for a known child, including its route."""
        _required(parent)
        _required(child)
        with closing(self._connect()) as db, db:
            row = db.execute(
                "SELECT * FROM reservations WHERE parent=? AND child=?", (parent, child)
            ).fetchone()
            return dict(row) if row else None

    @staticmethod
    def _row(db: sqlite3.Connection, parent: str, request: str) -> sqlite3.Row:
        row = db.execute(
            "SELECT * FROM reservations WHERE parent=? AND request=?",
            (_required(parent), _required(request)),
        ).fetchone()
        if row is None:
            raise ReservationError("unknown reservation")
        return row
