import json
import shutil
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).parents[3]
PLUGIN_FILES = tuple(
    "skills/dreaming/scripts/resumable-context-capsule.sh skills/dreaming/scripts/resumable-context-capsule.cmd skills/dreaming/scripts/resumable_context_capsule.py".split()
)
RUNTIME_SCHEMA = ROOT / "packages/codexy-runtime/schemas/handoff-runtime.schema.json"


# Keep component packaging checks separate from native runtime scenarios.
class ResumableContextCapsuleSourceTests(unittest.TestCase):
    def test_component_sources_are_installed_and_manifest_declares_them(self) -> None:
        self.assertTrue(
            RUNTIME_SCHEMA.is_file(), "missing runtime-owned handoff schema"
        )
        with tempfile.TemporaryDirectory() as temporary:
            plugin = Path(temporary) / "plugins" / "codexy"
            shutil.copytree(ROOT / "plugins/codexy", plugin)
            missing = [item for item in PLUGIN_FILES if not (plugin / item).is_file()]
            self.assertEqual(
                missing, [], f"missing installed capsule sources: {missing}"
            )
        manifest_path = ROOT / (
            "packages/getcodexy/src/codexy_runtime_tools/component-manifest.json"
        )
        manifest = json.loads(manifest_path.read_text())
        core = next(item for item in manifest["components"] if item["id"] == "core")
        required = core["asset"]["requiredPaths"]
        self.assertEqual(set(PLUGIN_FILES) - set(required), set())
        generated = lambda item: item.startswith(("handoff-runtime.json", "runtime/"))
        self.assertFalse(any(map(generated, required)))
