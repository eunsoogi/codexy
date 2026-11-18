"""Small JSON host mutation adapters shared by lifecycle recovery."""

from pathlib import Path
from .component_lifecycle_preflight import Runner
from .component_manifest import ComponentManifest
from .pre_session import _json


def mutate(
    executable: Path,
    invoke: Runner,
    action: str,
    manifest: ComponentManifest,
    component_id: str,
) -> None:
    _json(
        invoke(
            [
                str(executable),
                "plugin",
                action,
                manifest.component(component_id).asset.plugin_id,
                "--json",
            ]
        ),
        f"plugin {action}",
    )


def list_installed(executable: Path, invoke: Runner) -> object:
    return _json(invoke([str(executable), "plugin", "list", "--json"]), "plugin list")
