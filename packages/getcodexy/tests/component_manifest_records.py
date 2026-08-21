"""Installed-component records for manifest resolver cases."""

from codexy_runtime_tools.component_manifest import load_component_manifest


VERSION = load_component_manifest().version


def installed(plugin: str, version: str = VERSION) -> dict[str, object]:
    return {
        "pluginId": f"{plugin}@codexy",
        "name": plugin,
        "marketplaceName": "codexy",
        "version": version,
        "installed": True,
        "enabled": True,
        "source": {"source": "local", "path": f"/marketplace/plugins/{plugin}"},
        "marketplaceSource": {
            "sourceType": "git",
            "source": "https://github.com/eunsoogi/codexy.git",
        },
    }
