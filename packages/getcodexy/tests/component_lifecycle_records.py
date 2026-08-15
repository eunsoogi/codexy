"""Installed-component inventory record helpers."""

import json
from pathlib import Path

from codexy_runtime_tools.component_lifecycle import inventory_path


def record(home: Path, components: list[str]) -> None:
    target = inventory_path(home)
    target.parent.mkdir(parents=True)
    target.write_text(
        json.dumps(
            {
                "schema": "getcodexy.installed-component-inventory.v1",
                "components": components,
            }
        ),
        encoding="utf-8",
    )


def recorded(home: Path) -> list[str]:
    return json.loads(inventory_path(home).read_text(encoding="utf-8"))["components"]
