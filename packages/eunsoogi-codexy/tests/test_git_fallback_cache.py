import shutil
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from codexy_runtime_tools import runtime
from runtime_fixture import configuration, install_paths


class Executed(BaseException):
    pass


class GitFallbackCacheTests(unittest.TestCase):
    def test_rejected_cache_is_repaired_then_reused_offline(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            cache = root / "cache"
            config = configuration(root, git_fallback=True)
            installed, marker = install_paths(config, cache)
            installed.parent.mkdir(parents=True)
            installed.write_text("stale runtime", encoding="utf-8")
            installed.chmod(0o755)

            def repair(*_: object) -> None:
                installed.write_text("repaired runtime", encoding="utf-8")
                installed.chmod(0o755)
                shutil.copyfile(config.manifest, marker)

            with (
                mock.patch.object(runtime, "_cache_root", return_value=cache),
                mock.patch.object(runtime, "install_package", side_effect=RuntimeError("404")),
                mock.patch.object(runtime, "install_git", side_effect=repair) as install_git,
                mock.patch.object(runtime, "execute", side_effect=Executed),
                self.assertRaises(Executed),
            ):
                runtime.run(config)
            install_git.assert_called_once()

            offline = configuration(root, git_fallback=True, offline=True)
            with (
                mock.patch.object(runtime, "_cache_root", return_value=cache),
                mock.patch.object(runtime, "install_package") as install_package,
                mock.patch.object(runtime, "install_git") as offline_git,
                mock.patch.object(runtime, "execute", side_effect=Executed),
                self.assertRaises(Executed),
            ):
                runtime.run(offline)
            install_package.assert_not_called()
            offline_git.assert_not_called()


if __name__ == "__main__":
    unittest.main()
