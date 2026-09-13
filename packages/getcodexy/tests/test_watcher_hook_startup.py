from __future__ import annotations

import json
import os
import shlex
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
PLUGIN = ROOT / "plugins/codexy"


class WatcherHookStartupTests(unittest.TestCase):
    @unittest.skipUnless(os.name != "nt", "POSIX launcher coverage")
    def test_runtime_preserves_watcher_state_environment_precedence(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "codexy"
            shutil.copytree(PLUGIN, root)
            runtime_dir = root / "runtime"
            runtime_dir.mkdir()
            runtime_name = (
                "codexy-mcp-watcher-darwin-arm64.bin"
                if sys.platform == "darwin"
                else "codexy-mcp-watcher-linux-x86_64.bin"
            )
            fake_runtime = runtime_dir / runtime_name
            fake_runtime.write_text(
                "#!/usr/bin/env python3\n"
                "import json, os\n"
                "from pathlib import Path\n"
                "Path(os.environ['PLUGIN_ROOT'], 'captured.json').write_text(\n"
                "    json.dumps({key: os.environ.get(key, '<missing>') for key in (\n"
                "        'CODEXY_WATCHER_STATE_DIR', 'XDG_STATE_HOME', 'HOME'\n"
                "    )})\n"
                ")\n"
                "print('{\"cancelled\":false}')\n",
                encoding="utf-8",
            )
            fake_runtime.chmod(0o755)
            runtime = root / "hooks/codexy-hook-runtime.sh"
            runtime.write_text(
                runtime.read_text(encoding="utf-8").replace(
                    "for candidate in /usr/local/bin/python3 /usr/bin/python3; do",
                    f"for candidate in {shlex.quote(sys.executable)}; do",
                ),
                encoding="utf-8",
            )
            home = root / "home"
            xdg = root / "xdg"
            override = root / "override"
            for directory in (home, xdg, override):
                directory.mkdir()

            for state_override, expected in (
                (None, "<missing>"),
                (override, str(override)),
            ):
                environment = {
                    **os.environ,
                    "PLUGIN_ROOT": str(root),
                    "CODEXY_RUNTIME_DIR": str(runtime_dir),
                    "HOME": str(home),
                    "USER": "tester",
                    "XDG_STATE_HOME": str(xdg),
                }
                if state_override is None:
                    environment.pop("CODEXY_WATCHER_STATE_DIR", None)
                else:
                    environment["CODEXY_WATCHER_STATE_DIR"] = str(state_override)
                result = subprocess.run(
                    [str(root / "hooks/codexy-watcher-interrupt.sh"), "Interrupt"],
                    input=b'{"hook_event_name":"Interrupt","session_id":"main","turn_id":"turn"}',
                    capture_output=True,
                    env=environment,
                    check=False,
                )
                self.assertEqual(
                    (result.returncode, result.stdout, result.stderr), (0, b"", b"")
                )
                captured = json.loads(
                    (root / "captured.json").read_text(encoding="utf-8")
                )
                self.assertEqual(captured["CODEXY_WATCHER_STATE_DIR"], expected)
                self.assertEqual(captured["XDG_STATE_HOME"], str(xdg))
                self.assertEqual(captured["HOME"], str(home))


if __name__ == "__main__":
    unittest.main()
