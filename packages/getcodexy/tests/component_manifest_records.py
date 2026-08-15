"""Installed-component records for manifest resolver cases."""


def installed(plugin: str, version: str = "1.3.0") -> dict[str, object]:
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
