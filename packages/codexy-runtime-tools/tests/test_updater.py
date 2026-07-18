import contextlib
import io
import tempfile
import unittest
from pathlib import Path

from codexy_runtime_tools.updater import sync_agents


class UpdateApiTests(unittest.TestCase):
    def test_check_returns_structure_without_unsolicited_stdout(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            plugin_root = root / "plugin"
            codex_home = root / "codex-home"
            plugin_agents = plugin_root / "agents"
            installed_agents = codex_home / "agents" / "codexy"
            plugin_agents.mkdir(parents=True)
            (plugin_root / ".codex-plugin").mkdir()
            (plugin_root / ".codex-plugin" / "plugin.json").write_text(
                '{"name":"codexy"}', encoding="utf-8"
            )
            installed_agents.mkdir(parents=True)
            source = 'name = "codexy-sentinel"\n'
            (plugin_agents / "codexy-sentinel.toml").write_text(source, encoding="utf-8")
            (installed_agents / "codexy-sentinel.toml").write_text(
                "# CODEXY MANAGED AGENT\n" + source, encoding="utf-8"
            )
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                result = sync_agents(plugin_root, codex_home, "check")

            self.assertEqual(output.getvalue(), "")
            self.assertEqual(result.status, "ready")
            self.assertFalse(result.restart_required)


if __name__ == "__main__":
    unittest.main()
