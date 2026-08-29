from __future__ import annotations

import json
import subprocess
import tempfile
from pathlib import Path

from codexy_runtime_tools.component_manifest import load_component_manifest


OFFICIAL = "https://github.com/eunsoogi/codexy.git"
VERSION = load_component_manifest().version


class fixture:
    def __init__(
        self,
        selection: set[str] | None = None,
        *,
        fail_add: str | None = None,
        fail_remove: str | None = None,
        fail_upgrade: bool = False,
        interrupt_add: str | None = None,
        marketplace_present: bool = True,
        inventory_override: object | None = None,
        inventory_responses: list[object] | None = None,
        versions: dict[str, str] | None = None,
    ) -> None:
        (
            self.selection,
            self.fail_add,
            self.fail_remove,
            self.fail_upgrade,
            self.interrupt_add,
            self.marketplace_present,
            self.inventory_override,
            self.inventory_responses,
            self.versions,
        ) = (
            selection or set(),
            fail_add,
            fail_remove,
            fail_upgrade,
            interrupt_add,
            marketplace_present,
            inventory_override,
            list(inventory_responses or ()),
            versions or {},
        )
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name).resolve()
        self.home, self.marketplace, self.calls, self.mutations = (
            self.root / "home",
            self.root / "marketplace",
            [],
            [],
        )
        self.tag_revision: str | None = None
        self.main_revision: str | None = None
        self.home.mkdir()
        self.marketplace.mkdir()
        (self.home / "config.toml").write_text(
            f'[marketplaces.codexy]\nref = "v{VERSION}"\n', encoding="utf-8"
        )
        self.codex = self.root / "trusted/codex"
        self.codex.parent.mkdir(parents=True)
        self.codex.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
        self.codex.chmod(0o700)

    def __enter__(self) -> "fixture":
        return self

    def __exit__(self, *_: object) -> None:
        self.temporary.cleanup()

    def run(self, command: list[str]) -> subprocess.CompletedProcess[str]:
        tail = tuple(command[1:])
        self.calls.append(tail)
        if tail == ("plugin", "marketplace", "list", "--json"):
            entries = (
                []
                if not self.marketplace_present
                else [
                    {
                        "name": "codexy",
                        "root": str(self.marketplace),
                        "marketplaceSource": {"sourceType": "git", "source": OFFICIAL},
                    }
                ]
            )
            payload: object = {"marketplaces": entries}
        elif tail[:3] == ("plugin", "marketplace", "add"):
            self.marketplace_present = True
            self.mutations.append(tail)
            payload = {"ok": True}
        elif tail[:3] == ("plugin", "marketplace", "remove"):
            self.marketplace_present = False
            self.mutations.append(tail)
            payload = {"ok": True}
        elif tail[:3] == ("plugin", "marketplace", "upgrade"):
            self.mutations.append(tail)
            self._ensure_marketplace_identity()
            if self.fail_upgrade:
                self.fail_upgrade = False
                return subprocess.CompletedProcess(command, 1, "", "failed")
            self.versions = {component: VERSION for component in self.selection}
            payload = {"ok": True}
        elif tail == ("plugin", "list", "--json"):
            payload = (
                self.inventory_responses.pop(0)
                if self.inventory_responses
                else self.inventory_override
                if self.inventory_override is not None
                else {
                    "installed": [
                        installed(
                            self.marketplace,
                            component,
                            self.versions.get(component, VERSION),
                        )
                        for component in ("core", "github", "devtools")
                        if component in self.selection
                    ]
                }
            )
        elif tail[:2] == ("plugin", "add"):
            plugin = tail[2].split("@", 1)[0]
            self.selection.add(component_id(plugin))
            self.versions[component_id(plugin)] = VERSION
            self.mutations.append(tail)
            if plugin == self.interrupt_add:
                raise KeyboardInterrupt()
            if plugin == self.fail_add:
                self.fail_add = None
                return subprocess.CompletedProcess(command, 1, "", "failed")
            payload = {"ok": True}
        elif tail[:2] == ("plugin", "remove"):
            plugin = tail[2].split("@", 1)[0]
            if plugin == self.fail_remove:
                self.mutations.append(tail)
                self.fail_remove = None
                return subprocess.CompletedProcess(command, 1, "", "failed")
            self.selection.discard(component_id(plugin))
            self.versions.pop(component_id(plugin), None)
            self.mutations.append(tail)
            payload = {"ok": True}
        else:
            self.mutations.append(tail)
            payload = {"ok": True}
        return subprocess.CompletedProcess(command, 0, json.dumps(payload), "")

    def _ensure_marketplace_identity(self) -> None:
        if self.tag_revision is not None:
            return
        _git(self.marketplace, "init", "-q")
        _git(self.marketplace, "branch", "-M", "main")
        _git(self.marketplace, "config", "user.name", "fixture")
        _git(self.marketplace, "config", "user.email", "fixture@example.invalid")
        (self.marketplace / "release-marker").write_text("release", encoding="utf-8")
        _git(self.marketplace, "add", "release-marker")
        _git(self.marketplace, "commit", "-qm", "fixture release")
        tag = f"v{VERSION}"
        _git(self.marketplace, "tag", tag)
        self.tag_revision = _git(self.marketplace, "rev-parse", f"{tag}^{{commit}}")
        (self.marketplace / "main-marker").write_text("main", encoding="utf-8")
        _git(self.marketplace, "add", "main-marker")
        _git(self.marketplace, "commit", "-qm", "fixture main drift")
        self.main_revision = _git(self.marketplace, "rev-parse", "main")
        _git(self.marketplace, "checkout", "-q", "--detach", tag)
        (self.marketplace / ".codex-marketplace-install.json").write_text(
            json.dumps(
                {
                    "ref_name": tag,
                    "revision": self.tag_revision,
                    "source": OFFICIAL,
                    "source_type": "git",
                    "sparse_paths": [],
                }
            ),
            encoding="utf-8",
        )


def component_id(plugin: str) -> str:
    return {"codexy": "core", "codexy-github": "github", "codexy-devtools": "devtools"}[
        plugin
    ]


def installed(root: Path, component: str, version: str = VERSION) -> dict[str, object]:
    plugin = {
        "core": "codexy",
        "github": "codexy-github",
        "devtools": "codexy-devtools",
    }[component]
    return {
        "pluginId": f"{plugin}@codexy",
        "name": plugin,
        "marketplaceName": "codexy",
        "version": version,
        "installed": True,
        "enabled": True,
        "source": {"source": "local", "path": str(root / "plugins" / plugin)},
        "marketplaceSource": {"sourceType": "git", "source": OFFICIAL},
    }


def _git(root: Path, *arguments: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(root), *arguments],
        text=True,
        capture_output=True,
        check=False,
    )
    if result.returncode:
        raise RuntimeError(result.stderr)
    return result.stdout.strip()
