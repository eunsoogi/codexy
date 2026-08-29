from __future__ import annotations

import json
import subprocess
from pathlib import Path


OFFICIAL = "https://github.com/eunsoogi/codexy.git"


def commands() -> list[tuple[str, ...]]:
    return [
        reconcile_marketplace_commands()[0],
        ("/trusted/codex", "plugin", "list", "--json"),
        *reconcile_marketplace_commands()[1:],
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


def reconcile_marketplace_commands() -> list[tuple[str, ...]]:
    return [
        ("/trusted/codex", "plugin", "marketplace", "list", "--json"),
        (
            "/trusted/codex",
            "plugin",
            "marketplace",
            "remove",
            "codexy",
            "--json",
        ),
        (
            "/trusted/codex",
            "plugin",
            "marketplace",
            "add",
            "eunsoogi/codexy",
            "--ref",
            "v1.2.2",
            "--json",
        ),
        ("/trusted/codex", "plugin", "marketplace", "list", "--json"),
    ]


def fresh_marketplace_commands() -> list[tuple[str, ...]]:
    return [
        ("/trusted/codex", "plugin", "marketplace", "list", "--json"),
        (
            "/trusted/codex",
            "plugin",
            "marketplace",
            "add",
            "eunsoogi/codexy",
            "--ref",
            "v1.2.2",
            "--json",
        ),
        ("/trusted/codex", "plugin", "marketplace", "list", "--json"),
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
    if root.parent.name == "plugins" and root.name == "codexy":
        marketplace_root = root.parent.parent
        _git(marketplace_root, "init", "-q")
        _git(marketplace_root, "branch", "-M", "main")
        _git(marketplace_root, "config", "user.name", "fixture")
        _git(marketplace_root, "config", "user.email", "fixture@example.invalid")
        _git(marketplace_root, "add", ".")
        _git(marketplace_root, "commit", "-qm", "fixture main")
        (marketplace_root / "release-marker").write_text("tag", encoding="utf-8")
        _git(marketplace_root, "add", "release-marker")
        _git(marketplace_root, "commit", "-qm", "fixture release")
        _git(marketplace_root, "tag", "v1.2.2")
        tag_revision = _git(marketplace_root, "rev-parse", "v1.2.2^{commit}")
        _git(marketplace_root, "checkout", "-q", "--detach", "v1.2.2")
        (marketplace_root / ".codex-marketplace-install.json").write_text(
            json.dumps(
                {
                    "ref_name": "v1.2.2",
                    "revision": tag_revision,
                    "source": OFFICIAL,
                    "source_type": "git",
                    "sparse_paths": [],
                }
            ),
            encoding="utf-8",
        )
    return root


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
