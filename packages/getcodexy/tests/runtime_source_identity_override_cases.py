"""Override-source runtime identity cases."""

import hashlib
import json
import os
import shutil
import tempfile
from pathlib import Path
from unittest import mock

from codexy_runtime_tools import package
from codexy_runtime_tools import runtime


class RuntimeSourceIdentityOverrideCases:
    def test_all_sha_pinned_override_sources_are_independent_and_cache_isolated(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self._write_selected_release(root)
            archive = root / "override.tar.gz"
            self._write_override(archive)
            digest = hashlib.sha256(archive.read_bytes()).hexdigest()
            for source_name, source_key, source in (
                ("path", "CODEXY_RUNTIME_PACKAGE_PATH", str(archive)),
                (
                    "url",
                    "CODEXY_RUNTIME_PACKAGE_URL",
                    "https://example.test/override.tar.gz",
                ),
                (
                    "artifacts",
                    "CODEXY_RUNTIME_ARTIFACTS_API_URL",
                    "https://api.github.com/repos/eunsoogi/codexy/actions/artifacts",
                ),
            ):
                with self.subTest(source=source_name):
                    cache = root / f"cache-{source_name}"
                    environment = {
                        "CODEXY_RUNTIME_CACHE_DIR": str(cache),
                        source_key: source,
                        "CODEXY_RUNTIME_PACKAGE_SHA256": digest,
                        "CODEXY_RUNTIME_PLATFORM": "linux-x86_64",
                    }
                    download = lambda _url, destination, _token="": shutil.copyfile(
                        archive, destination
                    )
                    with (
                        mock.patch.dict(os.environ, environment, clear=True),
                        mock.patch.object(package, "_download", side_effect=download),
                        mock.patch.object(
                            package, "_artifact_package", return_value=archive
                        ),
                        mock.patch.object(runtime, "_execute", side_effect=Executed),
                        self.assertRaises(Executed),
                    ):
                        runtime.run(runtime.Configuration.load("lsp", root, []))
                    marker = next(cache.rglob("runtime-marker.json"))
                    self.assertEqual(
                        json.loads(marker.read_text())["identity"]["mode"],
                        "explicit-override",
                    )
                    with (
                        mock.patch.dict(
                            os.environ, {**environment, "UV_OFFLINE": "1"}, clear=True
                        ),
                        mock.patch.object(runtime, "_execute", side_effect=Executed),
                        self.assertRaises(Executed),
                    ):
                        runtime.run(runtime.Configuration.load("lsp", root, []))
                    with (
                        mock.patch.dict(
                            os.environ,
                            {
                                "CODEXY_RUNTIME_CACHE_DIR": str(cache),
                                "UV_OFFLINE": "1",
                                "CODEXY_RUNTIME_PLATFORM": "linux-x86_64",
                            },
                            clear=True,
                        ),
                        mock.patch.object(runtime, "_execute") as execute,
                        self.assertRaisesRegex(SystemExit, "127"),
                    ):
                        runtime.run(runtime.Configuration.load("lsp", root, []))
                    execute.assert_not_called()


class Executed(BaseException):
    pass
