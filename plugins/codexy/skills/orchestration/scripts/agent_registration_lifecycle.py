"""No-follow, rollback-safe filesystem lifecycle for Codexy agent projection."""

from __future__ import annotations

import datetime as _dt
import os
from contextlib import contextmanager
from pathlib import Path

from agent_registration_fs import (
    MANAGED,
    FileState,
    Transaction,
    basename,
    directory_path,
    ensure_directory_path,
    snapshot,
    trusted_path,
)

LOCK = ".codexy-agent-registration.lock"


class RegistrationStore:
    def __init__(self, discovery_home: Path, config_name: str):
        basename(config_name)
        self.home = trusted_path(discovery_home)
        self.config_name = config_name
        self.agents_parent = self.home / "agents"
        self.agents_root = self.agents_parent / "codexy"
        self._created: list[Path] = []
        self._validate_layout()

    def __enter__(self) -> "RegistrationStore":
        return self

    def __exit__(self, *_args) -> None:
        self.close()

    def close(self) -> None:
        pass

    def config_text(self) -> str:
        state = snapshot(self.home / self.config_name)
        return state.data.decode("utf-8") if state.data is not None else ""

    def agent_texts(self) -> dict[str, str]:
        return {
            name: state.data.decode("utf-8")
            for name, state in self._agent_states().items()
            if state.data is not None
        }

    def validate_install(self, projections: dict[str, str]) -> None:
        states = self._agent_states()
        for name, contents in projections.items():
            basename(name)
            state = states.get(name, FileState(None))
            if state.data not in (None, contents.encode("utf-8")) and not state.data.startswith(
                MANAGED
            ):
                raise ValueError(
                    f"{self.agents_root / name} is not owned by Codexy; "
                    "move or remove it before registration"
                )

    def install(
        self, projections: dict[str, str], original_config: str, new_config: str | None
    ) -> None:
        self._ensure_home()
        try:
            with self._lock():
                self._ensure_layout()
                if self.config_text() != original_config:
                    raise RuntimeError("config changed during registration; retry")
                self.validate_install(projections)
                states = self._agent_states()
                transaction = Transaction()
                try:
                    for name in sorted(states):
                        state = states[name]
                        if name not in projections and state.data and state.data.startswith(MANAGED):
                            transaction.delete(self.agents_root / name, state)
                    for name, contents in projections.items():
                        transaction.write(
                            self.agents_root / name,
                            contents.encode("utf-8"),
                            states.get(name, FileState(None)),
                        )
                    self._rewrite_config(transaction, original_config, new_config)
                    transaction.commit()
                except Exception:
                    transaction.rollback()
                    raise
        except Exception:
            self._cleanup_created()
            raise

    def uninstall(self, original_config: str, new_config: str | None) -> int:
        if not os.path.lexists(self.home):
            return 0
        try:
            with self._lock():
                if self.config_text() != original_config:
                    raise RuntimeError("config changed during uninstall; retry")
                states = self._agent_states()
                managed = [
                    name
                    for name, state in states.items()
                    if state.data and state.data.startswith(MANAGED)
                ]
                if not managed and new_config is None:
                    return 0
                transaction = Transaction()
                try:
                    for name in sorted(managed):
                        transaction.delete(self.agents_root / name, states[name])
                    self._rewrite_config(transaction, original_config, new_config)
                    transaction.commit()
                    self._remove_empty_agent_dirs()
                except Exception:
                    transaction.rollback()
                    raise
        except Exception:
            self._cleanup_created()
            raise
        return len(managed)

    def _rewrite_config(
        self, transaction: Transaction, original: str, updated: str | None
    ) -> None:
        if updated is None:
            return
        config = self.home / self.config_name
        current = snapshot(config)
        if current.data != original.encode("utf-8"):
            raise RuntimeError("config changed during registration; retry")
        backup = self.home / self._backup_name()
        transaction.write(backup, original.encode("utf-8"), FileState(None), current.mode)
        transaction.write(config, updated.encode("utf-8"), current, current.mode)

    def _backup_name(self) -> str:
        stamp = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%d%H%M%S")
        base = f"{self.config_name}.codexy-backup-{stamp}"
        name, counter = base, 1
        while os.path.lexists(self.home / name):
            name, counter = f"{base}-{counter}", counter + 1
        return name

    def _agent_states(self) -> dict[str, FileState]:
        self._validate_layout()
        if not self.agents_root.exists():
            return {}
        states = {}
        for entry in os.scandir(self.agents_root):
            if entry.name.endswith(".toml"):
                basename(entry.name)
                states[entry.name] = snapshot(Path(entry.path))
        return states

    def _validate_layout(self) -> None:
        for path in (self.home.parent, self.home, self.agents_parent, self.agents_root):
            directory_path(path, missing_ok=True)

    def _ensure_home(self) -> None:
        self._created.extend(ensure_directory_path(self.home))
        directory_path(self.home)

    def _ensure_layout(self) -> None:
        self._ensure_home()
        self._created.extend(ensure_directory_path(self.agents_root))
        self._validate_layout()

    @contextmanager
    def _lock(self):
        path = self.home / LOCK
        if os.path.lexists(path):
            raise RuntimeError(f"registration lock already exists: {path}")
        flags = os.O_CREAT | os.O_EXCL | os.O_WRONLY | getattr(os, "O_NOFOLLOW", 0)
        descriptor = os.open(path, flags, 0o600)
        identity = os.fstat(descriptor)
        try:
            os.write(descriptor, f"{os.getpid()}\n".encode())
            yield
        finally:
            os.close(descriptor)
            try:
                current = os.lstat(path)
                if (current.st_dev, current.st_ino) == (identity.st_dev, identity.st_ino):
                    path.unlink()
            except FileNotFoundError:
                pass

    def _cleanup_created(self) -> None:
        for path in reversed(self._created):
            try:
                path.rmdir()
            except OSError:
                pass
        self._created.clear()

    def _remove_empty_agent_dirs(self) -> None:
        for path in (self.agents_root, self.agents_parent):
            try:
                path.rmdir()
            except OSError:
                break
