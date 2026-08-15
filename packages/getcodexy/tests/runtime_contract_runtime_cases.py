"""Runtime installation boundary cases for release contracts."""

import hashlib
import importlib
import os
from pathlib import Path
from unittest import mock

from runtime_contract_support import COMMIT, legacy, release


class RuntimeContractRuntimeCases:
    def test_explicit_override_cannot_poison_selected_release_cache(self) -> None:
        root, _ = self.load(legacy())
        runtime = importlib.import_module("codexy_runtime_tools.runtime")
        cache = root / "cache"
        override = root / "override.tar.gz"
        override.write_bytes(b"override archive")
        installed_roots: list[Path] = []

        def install_override(_config, install_root: Path, installed: Path) -> None:
            installed.parent.mkdir(parents=True)
            installed.write_bytes(b"override controlled runtime")
            installed.chmod(0o755)
            installed_roots.append(install_root)

        environment = {
            "CODEXY_RUNTIME_CACHE_DIR": str(cache),
            "CODEXY_RUNTIME_PACKAGE_PATH": str(override),
            "CODEXY_RUNTIME_PACKAGE_SHA256": hashlib.sha256(
                override.read_bytes()
            ).hexdigest(),
        }
        with (
            mock.patch.dict(os.environ, environment, clear=True),
            mock.patch.object(runtime, "install_package", side_effect=install_override),
            mock.patch.object(runtime, "_execute", side_effect=SystemExit(0)),
            self.assertRaises(SystemExit),
        ):
            runtime.run(runtime.Configuration.load("lsp", root, []))
        self.assertEqual(len(installed_roots), 1)

        with (
            mock.patch.dict(
                os.environ,
                {"CODEXY_RUNTIME_CACHE_DIR": str(cache), "UV_OFFLINE": "1"},
                clear=True,
            ),
            mock.patch.object(runtime, "_execute") as execute,
            self.assertRaises(SystemExit) as failure,
        ):
            runtime.run(runtime.Configuration.load("lsp", root, []))
        self.assertEqual(failure.exception.code, 127)
        execute.assert_not_called()

    def test_runtime_boundary_never_invokes_cargo_without_explicit_exact_fallback(
        self,
    ) -> None:
        root, _ = self.load(release())
        runtime = importlib.import_module("codexy_runtime_tools.runtime")
        environment = {"CODEXY_RUNTIME_CACHE_DIR": str(root / "cache")}
        with (
            mock.patch.dict(os.environ, environment, clear=True),
            mock.patch.object(
                runtime,
                "install_package",
                side_effect=RuntimeError("missing public artifact"),
            ),
            mock.patch.object(runtime, "install_git") as cargo,
            self.assertRaises(SystemExit),
        ):
            runtime.run(runtime.Configuration.load("lsp", root, []))
        cargo.assert_not_called()
        with (
            mock.patch.dict(
                os.environ,
                {**environment, "CODEXY_RUNTIME_GIT_FALLBACK": "1"},
                clear=True,
            ),
            mock.patch.object(
                runtime,
                "install_package",
                side_effect=RuntimeError("missing public artifact"),
            ),
            mock.patch.object(
                runtime, "install_git", side_effect=RuntimeError("cargo failed")
            ) as cargo,
            self.assertRaises(SystemExit),
        ):
            configuration = runtime.Configuration.load("lsp", root, [])
            self.assertEqual(configuration.git_ref, COMMIT)
            runtime.run(configuration)
        cargo.assert_called_once()
