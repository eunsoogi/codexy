import json
import subprocess
import tempfile
import unittest
from pathlib import Path

from codexy_runtime_tools.cache import runtime_cache_key


REPOSITORY = Path(__file__).resolve().parents[3]


class McpCacheRootTests(unittest.TestCase):
    def test_relative_home_never_executes_cached_runtime(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bin_dir = root / "bin"
            bin_dir.mkdir()
            uname = bin_dir / "uname"
            uname.write_text('#!/bin/sh\n[ "$1" = "-s" ] && echo Linux || echo x86_64\n')
            uname.chmod(0o755)
            manifest = REPOSITORY / "plugins" / "codexy" / ".codex-plugin" / "plugin.json"
            for server in ("lsp", "codegraph"):
                key = runtime_cache_key(
                    manifest=manifest,
                    package_override=False,
                    identity=["https://github.com/eunsoogi/codexy", "", "linux-x86_64", "stdio-newline-v1", "package-default\n", f"codexy-mcp-{server}"],
                )
                cached = root / ".cache" / "codexy" / "runtime" / key / "bin" / f"codexy-mcp-{server}"
                cached.parent.mkdir(parents=True)
                cached.write_text('#!/bin/sh\necho cached-runtime\n')
                cached.chmod(0o755)
                (cached.parents[1] / "plugin.json").write_text(json.dumps({"version": "1.2.1"}))
                wrapper = REPOSITORY / "plugins" / "codexy" / "mcp" / f"codexy-mcp-{server}"
                completed = subprocess.run(
                    [wrapper], cwd=root,
                    env={"PATH": f"{bin_dir}:/usr/bin:/bin", "HOME": ".", "UV_OFFLINE": "1"},
                    capture_output=True, text=True,
                )
                self.assertEqual(completed.returncode, 127)
                self.assertNotIn("cached-runtime", completed.stdout)
