from __future__ import annotations

import json
import subprocess
from pathlib import Path


OFFICIAL = "https://github.com/eunsoogi/codexy.git"


def commands() -> list[tuple[str, ...]]:
    return [
        ("/trusted/codex", "plugin", "marketplace", "list", "--json"),
        ("/trusted/codex", "plugin", "list", "--json"),
        (
            "/trusted/codex",
            "plugin",
            "marketplace",
            "upgrade",
            "codexy",
            "--json",
        ),
        ("/trusted/codex", "plugin", "marketplace", "list", "--json"),
        ("/trusted/codex", "plugin", "add", "codexy@codexy", "--json"),
        ("/trusted/codex", "plugin", "list", "--json"),
    ]


def respond(
    command: list[str],
    calls: list[tuple[str, ...]],
    before: list[dict[str, object]],
    after: list[dict[str, object]],
    marketplace_root: Path,
) -> subprocess.CompletedProcess[str]:
    calls.append(tuple(command))
    command_tuple = tuple(command)
    if command_tuple[1:4] == ("plugin", "marketplace", "list"):
        payload: object = {"marketplaces": [marketplace(marketplace_root)]}
    elif command_tuple[1:3] == ("plugin", "list"):
        payload = {"installed": before if calls.count(command_tuple) == 1 else after}
    else:
        payload = {"ok": True}
    return subprocess.CompletedProcess(command, 0, json.dumps(payload), "")


def marketplace(root: Path) -> dict[str, object]:
    return {
        "name": "codexy",
        "root": str(root),
        "marketplaceSource": {"sourceType": "git", "source": OFFICIAL},
    }


def wrong_marketplace(root: Path) -> dict[str, object]:
    return {
        "name": "codexy",
        "root": str(root),
        "marketplaceSource": {
            "sourceType": "git",
            "source": "https://example.invalid/codexy.git",
        },
    }


def installed(root: Path) -> dict[str, object]:
    return {
        "pluginId": "codexy@codexy",
        "name": "codexy",
        "marketplaceName": "codexy",
        "version": "1.2.2",
        "installed": True,
        "enabled": True,
        "source": {"source": "local", "path": str(root)},
        "marketplaceSource": {
            "sourceType": "git",
            "source": OFFICIAL,
        },
    }


def make_plugin(root: Path) -> Path:
    manifest = root / ".codex-plugin" / "plugin.json"
    manifest.parent.mkdir(parents=True)
    manifest.write_text(
        '{"name":"codexy","repository":"https://github.com/eunsoogi/codexy",'
        '"version":"1.2.2"}',
        encoding="utf-8",
    )
    return root
