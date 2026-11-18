"""Synthetic local-upgrade host; plugin records retain their installed origin."""

from contextlib import ExitStack
from dataclasses import replace
import json
import shutil
import subprocess
from unittest.mock import patch

from codexy_runtime_tools.component_lifecycle import run_operation
from codexy_runtime_tools.component_lifecycle_mcp import materialize_mcp_sources
from codexy_runtime_tools.component_transaction_state import write_inventory
from codexy_runtime_tools.plugin_resolution import MarketplaceIdentity
from packages.getcodexy.tests.component_hook_registration_fixture import (
    fixture_hook_rows,
    registration_context,
)
from packages.getcodexy.tests.component_marketplace_fixture import (
    configure_core_package,
)
from packages.getcodexy.tests.local_marketplace_identity_fixture import (
    LocalHost,
    MANIFEST,
)


class UpgradeHost(LocalHost):
    """Keep host protocol synthetic; real_registration explicitly enables that gate."""

    def __init__(
        self, selection=("core", "github", "devtools"), *, real_registration=False
    ):
        super().__init__()
        self.old = self.marketplace
        self.target = self.home / "getcodexy/marketplaces/verified-target/marketplace"
        self.target.parent.mkdir(parents=True)
        shutil.copytree(self.old, self.target)
        metadata = self.old / ".agents/plugins/marketplace.json"
        data = json.loads(metadata.read_text())
        for entry in data["plugins"]:
            entry["version"] = "2.2.4"
            path = self.old / "plugins" / entry["name"] / ".codex-plugin/plugin.json"
            plugin = json.loads(path.read_text())
            plugin["version"] = "2.2.4"
            path.write_text(json.dumps(plugin))
        metadata.write_text(json.dumps(data))
        configure_core_package(self.old / "plugins/codexy", "2.2.4")
        old_manifest = replace(
            MANIFEST,
            version="2.2.4",
            components=tuple(replace(c, version="2.2.4") for c in MANIFEST.components),
        )
        materialize_mcp_sources(self.old, old_manifest, selection)
        self.selection = set(selection)
        self.records = {c: (self.old, "2.2.4") for c in selection}
        write_inventory(
            self.home, tuple(c for c in MANIFEST.component_ids if c in self.selection)
        )
        self.settings = b'model = "user-choice"\n[unrelated]\nkeep = true\n'
        (self.home / "config.toml").write_bytes(self.settings)
        self.present = True
        self.fail_add = None
        self.fail_marketplace_add = False
        self.before_add = None
        self.registration = real_registration
        self.context = ExitStack()

    def __enter__(self):
        self.context.enter_context(registration_context(self.registration))
        self.acquire = self.context.enter_context(
            patch(
                "codexy_runtime_tools.local_release_bundle.acquire_target",
                return_value=MarketplaceIdentity(
                    self.target, "local", str(self.target)
                ),
            )
        )
        return self

    def __exit__(self, *args):
        self.context.__exit__(*args)
        super().__exit__(*args)

    def operate(self, command="update", identifier="op-upgrade-local", requested=()):
        return run_operation(
            command,
            requested,
            self.home,
            self.codex,
            self.run,
            operation_id=identifier,
            hook_lister=lambda *_: fixture_hook_rows(self.marketplace),
        )

    def _installed(self, name, installed_source):
        component = self._component(name)
        root, version = self.records[component]
        record = super()._installed(name, installed_source)
        record.update(
            version=version,
            source={"source": "local", "path": str(root / "plugins" / name)},
            marketplaceSource={"sourceType": "local", "source": str(root)},
        )
        return record

    def run(self, command):
        tail = tuple(command[1:])
        if tail == ("plugin", "marketplace", "list", "--json") and not self.present:
            return subprocess.CompletedProcess(command, 0, '{"marketplaces": []}', "")
        if tail[:3] == ("plugin", "marketplace", "remove"):
            self.present = False
        elif tail[:3] == ("plugin", "marketplace", "add"):
            if self.fail_marketplace_add:
                self.fail_marketplace_add = False
                return subprocess.CompletedProcess(
                    command, 1, "", "injected registration failure"
                )
            self.marketplace = type(self.old)(tail[3])
            self.present = True
        elif tail[:2] == ("plugin", "add"):
            component = self._component(tail[2].split("@", 1)[0])
            if self.before_add:
                self.before_add(component)
            if self.fail_add == component:
                self.fail_add = None
                return subprocess.CompletedProcess(
                    command, 1, "", "injected add failure"
                )
            self.records[component] = (
                self.marketplace,
                "2.2.4" if self.marketplace == self.old else MANIFEST.version,
            )
            self.selection.add(component)
        elif tail[:2] == ("plugin", "remove"):
            component = self._component(tail[2].split("@", 1)[0])
            self.selection.discard(component)
            self.records.pop(component, None)
        else:
            return super().run(command)
        self.mutations.append(tail)
        return subprocess.CompletedProcess(command, 0, '{"ok": true}', "")
