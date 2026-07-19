import json
import subprocess
import tempfile
import unittest
from pathlib import Path

from codexy_runtime_tools.cache import runtime_cache_key


REPOSITORY = Path(__file__).resolve().parents[3]


class McpCacheRootTests(unittest.TestCase):
    def test_empty_repository_override_reuses_the_matching_cached_runtime(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bin_dir = root / "bin"
            bin_dir.mkdir()
            uname = bin_dir / "uname"
            uname.write_text('#!/bin/sh\n[ "$1" = "-s" ] && echo Linux || echo x86_64\n')
            uname.chmod(0o755)
            manifest = REPOSITORY / "plugins" / "codexy" / ".codex-plugin" / "plugin.json"
            cache = root / "cache"
            for server in ("lsp", "codegraph"):
                key = runtime_cache_key(
                    manifest=manifest,
                    package_override=False,
                    identity=["", "", "linux-x86_64", "stdio-newline-v1", "package-default\n", f"codexy-mcp-{server}"],
                )
                cached = cache / key / "bin" / f"codexy-mcp-{server}"
                cached.parent.mkdir(parents=True)
                cached.write_text('#!/bin/sh\necho cached-runtime\n')
                cached.chmod(0o755)
                (cached.parents[1] / "plugin.json").write_text(json.dumps({"version": "1.2.1"}))
                wrapper = REPOSITORY / "plugins" / "codexy" / "mcp" / f"codexy-mcp-{server}"
                completed = subprocess.run(
                    [wrapper],
                    env={"PATH": f"{bin_dir}:/usr/bin:/bin", "UV_OFFLINE": "1", "CODEXY_RUNTIME_CACHE_DIR": str(cache), "CODEXY_RUNTIME_GIT_REPOSITORY": ""},
                    capture_output=True,
                    text=True,
                )
                self.assertEqual(completed.returncode, 0, completed.stderr)
                self.assertEqual(completed.stdout.strip(), "cached-runtime")

    def test_uppercase_digest_reuses_the_matching_cached_runtime(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bin_dir = root / "bin"
            bin_dir.mkdir()
            uname = bin_dir / "uname"
            uname.write_text('#!/bin/sh\n[ "$1" = "-s" ] && echo Linux || echo x86_64\n')
            uname.chmod(0o755)
            manifest = REPOSITORY / "plugins" / "codexy" / ".codex-plugin" / "plugin.json"
            cache = root / "cache"
            for server in ("lsp", "codegraph"):
                key = runtime_cache_key(
                    manifest=manifest,
                    package_override=False,
                    identity=[
                        "https://github.com/eunsoogi/codexy", "", "linux-x86_64",
                        "stdio-newline-v1", f"package-default\n{'a' * 64}", f"codexy-mcp-{server}",
                    ],
                )
                cached = cache / key / "bin" / f"codexy-mcp-{server}"
                cached.parent.mkdir(parents=True)
                cached.write_text('#!/bin/sh\necho cached-runtime\n')
                cached.chmod(0o755)
                (cached.parents[1] / "plugin.json").write_text(json.dumps({"version": "1.2.1"}))
                wrapper = REPOSITORY / "plugins" / "codexy" / "mcp" / f"codexy-mcp-{server}"
                completed = subprocess.run(
                    [wrapper],
                    env={"PATH": f"{bin_dir}:/usr/bin:/bin", "UV_OFFLINE": "1", "CODEXY_RUNTIME_CACHE_DIR": str(cache), "CODEXY_RUNTIME_PACKAGE_SHA256": "A" * 64},
                    capture_output=True,
                    text=True,
                )
                self.assertEqual(completed.returncode, 0, completed.stderr)
                self.assertEqual(completed.stdout.strip(), "cached-runtime")

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
