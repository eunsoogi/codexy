from __future__ import annotations

import json
import shutil
import unittest
from dataclasses import replace
from pathlib import Path
from unittest.mock import patch

from codexy_runtime_tools.component_inspection import status
from codexy_runtime_tools.component_manifest import load_component_manifest
from packages.getcodexy.tests.component_hook_registration_fixture import hook_rows
from packages.getcodexy.tests.component_lifecycle_support import fixture

# Independent release inputs, not values derived from the checked-out release.
BEFORE, AFTER = "17.4.2", "18.6.3"
STALE = [{"code": "required-hook-trust-stale"}]


def install_registration(state, version: str, hooks: bytes) -> Path:
    plugin = state.marketplace / "plugins/codexy"
    manifest = plugin / ".codex-plugin/plugin.json"
    identity = json.loads(manifest.read_text(encoding="utf-8"))
    identity["version"] = version
    manifest.write_text(json.dumps(identity), encoding="utf-8")
    (plugin / "hooks/hooks.json").write_bytes(hooks)
    state.versions["core"] = version
    (state.home / "config.toml").write_text(
        f'[marketplaces.codexy]\nref = "v{version}"\n', encoding="utf-8"
    )
    return plugin


def cache_registration(state, version: str) -> Path:
    cache = state.home / "plugins/cache/codexy/codexy" / version
    shutil.copytree(state.marketplace / "plugins/codexy", cache)
    return cache


def inspect(state, version: str, cache: Path, trust: str = "trusted"):
    packaged = load_component_manifest()
    manifest = replace(
        packaged,
        version=version,
        components=tuple(
            replace(item, version=version) for item in packaged.components
        ),
    )
    rows = [
        {**row, "trustStatus": trust, "currentHash": "sha256:opaque-host-metadata"}
        for row in hook_rows(cache)
    ]
    with patch(
        "codexy_runtime_tools.component_inspection.load_component_manifest",
        return_value=manifest,
    ):
        return status(
            state.home,
            codex=state.codex,
            runner=state.run,
            hook_lister=lambda _executable, _home: rows,
        )["errors"]


class ComponentHookCacheTransitionTests(unittest.TestCase):
    def test_successive_future_registrations_accept_only_their_versioned_cache(self):
        with fixture({"core"}) as state:
            plugin = state.marketplace / "plugins/codexy"
            original = (plugin / "hooks/hooks.json").read_bytes()
            install_registration(state, BEFORE, original)
            previous = cache_registration(state, BEFORE)
            self.assertEqual(inspect(state, BEFORE, previous), [])

            install_registration(state, AFTER, original + b"\n")
            current = cache_registration(state, AFTER)
            self.assertEqual(inspect(state, AFTER, current), [])
            self.assertEqual(inspect(state, AFTER, previous), STALE)

            # Isolate cache-path identity from otherwise identical current bytes.
            shutil.rmtree(previous)
            previous = cache_registration(state, BEFORE)
            self.assertEqual(inspect(state, AFTER, previous), STALE)
            self.assertEqual(inspect(state, AFTER, current), [])
            self.assertEqual(state.mutations, [])

    def test_new_registration_rejects_old_bytes_wrong_identity_and_stale_trust(self):
        with fixture({"core"}) as state:
            plugin = state.marketplace / "plugins/codexy"
            original = (plugin / "hooks/hooks.json").read_bytes()
            install_registration(state, AFTER, original + b"\n")
            cache = cache_registration(state, AFTER)
            hooks = cache / "hooks/hooks.json"
            self.assertEqual(inspect(state, AFTER, cache), [])

            hooks.write_bytes(original)
            self.assertEqual(inspect(state, AFTER, cache), STALE)
            hooks.write_bytes(original + b"\n")

            path = cache / ".codex-plugin/plugin.json"
            identity = json.loads(path.read_text(encoding="utf-8"))
            for field, value in (
                ("version", BEFORE),
                ("name", "other-plugin"),
                ("repository", "https://example.invalid/other"),
            ):
                with self.subTest(identity=field):
                    path.write_text(
                        json.dumps({**identity, field: value}), encoding="utf-8"
                    )
                    self.assertEqual(inspect(state, AFTER, cache), STALE)
            path.write_text(json.dumps(identity), encoding="utf-8")
            for trust in ("modified", "stale"):
                with self.subTest(trust=trust):
                    self.assertEqual(inspect(state, AFTER, cache, trust), STALE)
            self.assertEqual(inspect(state, AFTER, cache), [])
            self.assertEqual(state.mutations, [])


if __name__ == "__main__":
    unittest.main()
