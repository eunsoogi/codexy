"""Bind a pre-authorized creation payload to at most one native tool call."""

from __future__ import annotations

import hashlib
import json
import sqlite3
from contextlib import closing
from typing import Any

from .supervision_reservations import ReservationError, Reservations, _required


def fingerprint(tool_input: object) -> str:
    """Hash canonical JSON without persisting prompts or other input contents."""
    if not isinstance(tool_input, dict):
        raise ReservationError("creation input must be an object")
    try:
        payload = json.dumps(
            tool_input,
            sort_keys=True,
            separators=(",", ":"),
            ensure_ascii=False,
            allow_nan=False,
        ).encode("utf-8")
    except (TypeError, ValueError, UnicodeError) as exc:
        raise ReservationError("invalid creation input") from exc
    if len(payload) > 1024 * 1024:
        raise ReservationError("creation input too large")
    return hashlib.sha256(payload).hexdigest()


class Dispatch:
    """Separate issue authority from the native call identifier assigned later."""

    def __init__(self, store: Reservations):
        self.store = store
        with closing(store._connect()) as db, db:
            db.execute("""CREATE TABLE IF NOT EXISTS supervision_calls (
                parent TEXT NOT NULL, request TEXT NOT NULL, call TEXT NOT NULL,
                closed INTEGER NOT NULL DEFAULT 0, PRIMARY KEY (parent, call))""")
            db.execute("""CREATE UNIQUE INDEX IF NOT EXISTS open_supervision_call
                ON supervision_calls(parent, request) WHERE closed=0""")
            db.execute("""CREATE TABLE IF NOT EXISTS supervision_parents (
                parent TEXT PRIMARY KEY)""")

    def bootstrap(self, parent: str, owners: list[dict[str, str]]) -> None:
        """Record the caller's complete, independently reconciled owner roster."""
        parent = _required(parent)
        if not isinstance(owners, list):
            raise ReservationError("owner roster must be a list")
        with closing(self.store._connect()) as db, db:
            db.execute("BEGIN IMMEDIATE")
            if db.execute(
                "SELECT 1 FROM supervision_parents WHERE parent=?", (parent,)
            ).fetchone():
                raise ReservationError("parent roster already reconciled")
            for owner in owners:
                if not isinstance(owner, dict) or set(owner) != {
                    "request",
                    "issue",
                    "model",
                    "thinking",
                    "child",
                }:
                    raise ReservationError("invalid owner roster")
                values = tuple(
                    _required(owner[key])
                    for key in ("request", "issue", "model", "thinking", "child")
                )
                try:
                    db.execute(
                        "INSERT INTO reservations(parent,request,issue,model,thinking,child,state) VALUES(?,?,?,?,?,?,'active')",
                        (parent, *values),
                    )
                except sqlite3.IntegrityError as exc:
                    raise ReservationError("conflicting owner roster") from exc
            # No dispatch can observe a partially imported roster.
            db.execute("INSERT INTO supervision_parents(parent) VALUES(?)", (parent,))

    def prepare(
        self, parent: str, issue: str, tool_input: dict[str, Any], limit: int = 8
    ) -> str:
        """Reserve a slot for these exact input bytes and requested model pair."""
        with closing(self.store._connect()) as db:
            if not db.execute(
                "SELECT 1 FROM supervision_parents WHERE parent=?", (_required(parent),)
            ).fetchone():
                raise ReservationError("parent owner roster has not been reconciled")
        model, thinking = tool_input.get("model"), tool_input.get("thinking")
        return self.store.reserve(
            parent,
            fingerprint(tool_input),
            issue,
            _required(model),
            _required(thinking),
            limit,
        )

    def claim(self, parent: str, call: str, tool_input: object) -> bool:
        """Consume a prepared request once; duplicate delivery of one call is safe."""
        parent, call, request = (
            _required(parent),
            _required(call),
            fingerprint(tool_input),
        )
        with closing(self.store._connect()) as db, db:
            db.execute("BEGIN IMMEDIATE")
            row = db.execute(
                "SELECT state,child FROM reservations WHERE parent=? AND request=?",
                (parent, request),
            ).fetchone()
            if row is None or row["state"] != "reserved" or row["child"] is not None:
                return False
            previous = db.execute(
                "SELECT request,closed FROM supervision_calls WHERE parent=? AND call=?",
                (parent, call),
            ).fetchone()
            if previous:
                return previous["request"] == request and previous["closed"] == 0
            existing = db.execute(
                "SELECT call FROM supervision_calls WHERE parent=? AND request=? AND closed=0",
                (parent, request),
            ).fetchone()
            if existing:
                return False
            try:
                db.execute(
                    "INSERT INTO supervision_calls(parent,request,call) VALUES(?,?,?)",
                    (parent, request, call),
                )
            except sqlite3.IntegrityError:
                return False
            return True

    def request_for_call(self, parent: str, call: str) -> str | None:
        with closing(self.store._connect()) as db:
            row = db.execute(
                "SELECT request FROM supervision_calls WHERE parent=? AND call=? AND closed=0",
                (_required(parent), _required(call)),
            ).fetchone()
            return row["request"] if row else None
