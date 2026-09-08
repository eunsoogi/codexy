"""Core Watcher source launchers exercise the real bootstrap selection path."""

from __future__ import annotations

import hashlib
import io
import json
import os
import shutil
import stat
import subprocess
import tarfile
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from codexy_runtime_tools.runtime_configuration import (
    PUBLIC_BUNDLE_ASSET,
    Configuration,
)
from codexy_runtime_tools.version_lock import default_package_version


REPOSITORY = "https://github.com/eunsoogi/codexy"
VERSION = default_package_version()


def write_bundle(path: Path) -> None:
    runtime = b'#!/bin/sh\nset -eu\nprintf \'%s\\n\' "$CODEXY_PLUGIN_ROOT" > "$CODEXY_EXECUTION_LOG"\nprintf \'%s\\n\' "$@" >> "$CODEXY_EXECUTION_LOG"\n'
    files = {
        "plugins/codexy/.codex-plugin/plugin.json": json.dumps(
            {"name": "codexy", "repository": REPOSITORY, "version": VERSION}
        ).encode(),
        "plugins/codexy/runtime/codexy-mcp-watcher-linux-x86_64.bin": runtime,
        "plugins/codexy-devtools/.codex-plugin/plugin.json": json.dumps(
            {
                "name": "codexy-devtools",
                "repository": REPOSITORY,
                "version": VERSION,
            }
        ).encode(),
    }
    with tarfile.open(path, "w:gz") as archive:
        for name, contents in files.items():
            member = tarfile.TarInfo(name)
            member.size = len(contents)
            member.mode = 0o755 if name.endswith(".bin") else 0o644
            archive.addfile(member, io.BytesIO(contents))


class CoreWatcherBootstrapTests(unittest.TestCase):
    def test_legacy_core_watcher_selects_the_public_bundle_asset(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest = root / ".codex-plugin/plugin.json"
            manifest.parent.mkdir()
            manifest.write_text(
                json.dumps(
                    {"name": "codexy", "repository": REPOSITORY, "version": VERSION}
                ),
                encoding="utf-8",
            )
            with mock.patch.dict(os.environ, {}, clear=True):
                config = Configuration.load("watcher", root, ["--stdio"])
            self.assertTrue(
                config.package_url.endswith(f"/v{VERSION}/{PUBLIC_BUNDLE_ASSET}")
            )
            self.assertTrue(config.allow_mixed_plugin_roots)
            self.assertEqual(config.source_identity.package_plugin_root(), "codexy")

    def test_source_launcher_runs_bootstrap_and_core_runtime(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            archive = root / "bundle.tar.gz"
            write_bundle(archive)
            result, plugin, source_log, execution_log = self.run_source_launcher(
                root, archive, package_override=True
            )
            self.assertEqual(
                result.returncode,
                0,
                f"{result.stderr}\n{result.stdout}\n{result.args}",
            )
            self.assertEqual(
                source_log.read_text().splitlines(),
                [str(root / "source-repository" / "packages/getcodexy")],
            )
            self.assertEqual(
                execution_log.read_text().splitlines(),
                [str(plugin.resolve()), "value with spaces"],
            )

    def test_source_launcher_downloads_default_public_bundle(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            archive = root / "bundle.tar.gz"
            write_bundle(archive)
            result, plugin, source_log, execution_log = self.run_source_launcher(
                root, archive, package_override=False
            )
            self.assertEqual(
                result.returncode,
                0,
                f"{result.stderr}\n{result.stdout}\n{result.args}",
            )
            self.assertEqual(
                source_log.read_text().splitlines(),
                [str(root / "source-repository" / "packages/getcodexy")],
            )
            self.assertEqual(
                execution_log.read_text().splitlines(),
                [str(plugin.resolve()), "value with spaces"],
            )

    @staticmethod
    def run_source_launcher(root: Path, archive: Path, *, package_override: bool):
        repository = root / "source-repository"
        plugin = repository / "plugins/codexy"
        package = repository / "packages/getcodexy"
        package.mkdir(parents=True)
        (package / "pyproject.toml").write_text("[project]\nname='getcodexy'\n")
        source_plugin = Path(__file__).parents[3] / "plugins/codexy"
        shutil.copytree(source_plugin, plugin)
        fake_bin = root / "bin"
        fake_bin.mkdir()
        source_log = root / "uvx-source.log"
        execution_log = root / "watcher-execution.log"
        fake_uvx = fake_bin / "uvx"
        fake_uvx.write_text(
            "#!/bin/sh\n"
            "set -eu\n"
            'test "$1" = --from\n'
            'source="$2"\n'
            'test -f "$source/pyproject.toml"\n'
            'printf \'%s\\n\' "$source" > "$CODEXY_UVX_SOURCE_LOG"\n'
            "shift 2\n"
            'test "$1" = codexy-mcp-runtime\n'
            "shift\n"
            'exec "$CODEXY_TEST_PYTHON" -c \'\n'
            "import os, shutil\n"
            "from codexy_runtime_tools import package\n"
            'def download(url, destination, token=""):\n'
            '    assert url == os.environ["CODEXY_TEST_EXPECTED_URL"]\n'
            '    shutil.copyfile(os.environ["CODEXY_TEST_BUNDLE"], destination)\n'
            "package._download = download\n"
            "from codexy_runtime_tools.runtime import main\n"
            "main()\n"
            '\' "$@"\n',
            encoding="utf-8",
        )
        fake_uvx.chmod(fake_uvx.stat().st_mode | stat.S_IXUSR)
        environment = os.environ.copy()
        environment.update(
            {
                "PATH": f"{fake_bin}:{environment.get('PATH', '')}",
                "PYTHONPATH": str(Path(__file__).parents[1] / "src"),
                "CODEXY_TEST_PYTHON": os.sys.executable,
                "CODEXY_TEST_BUNDLE": str(archive),
                "CODEXY_TEST_EXPECTED_URL": f"{REPOSITORY}/releases/download/v{VERSION}/{PUBLIC_BUNDLE_ASSET}",
                "CODEXY_UVX_SOURCE_LOG": str(source_log),
                "CODEXY_EXECUTION_LOG": str(execution_log),
                "CODEXY_RUNTIME_CACHE_DIR": str(root / "cache"),
                "CODEXY_RUNTIME_PLATFORM": "linux-x86_64",
            }
        )
        for name in (
            "CODEXY_RUNTIME_DIR",
            "CODEXY_RUNTIME_PACKAGE_URL",
            "CODEXY_RUNTIME_ARTIFACTS_API_URL",
            "CODEXY_RUNTIME_PACKAGE_SHA256",
        ):
            environment.pop(name, None)
        if package_override:
            environment.update(
                {
                    "CODEXY_RUNTIME_PACKAGE_PATH": str(archive),
                    "CODEXY_RUNTIME_PACKAGE_SHA256": hashlib.sha256(
                        archive.read_bytes()
                    ).hexdigest(),
                }
            )
        else:
            environment.pop("CODEXY_RUNTIME_PACKAGE_PATH", None)
        result = subprocess.run(
            [str(plugin / "mcp/codexy-mcp-watcher.sh"), "--stdio", "value with spaces"],
            env=environment,
            text=True,
            capture_output=True,
            check=False,
        )
        return result, plugin, source_log, execution_log


if __name__ == "__main__":
    unittest.main()
