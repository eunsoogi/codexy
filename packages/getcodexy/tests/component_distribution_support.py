from __future__ import annotations

import json
import shutil
from pathlib import Path

from codexy_runtime_tools.version_lock import default_package_version


COMPONENTS = {
    "codexy": "core",
    "codexy-github": "github",
    "codexy-devtools": "devtools",
}


def copy_marketplace_plugins(repository: Path, root: Path) -> str:
    version = default_package_version()
    for plugin in COMPONENTS:
        destination = root / "plugins" / plugin
        shutil.copytree(repository / "plugins" / plugin, destination)
        manifest_path = destination / ".codex-plugin/plugin.json"
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        manifest["version"] = version
        manifest_path.write_text(
            json.dumps(manifest, indent=2) + "\n", encoding="utf-8"
        )
    return version
