import os
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

from codexy_runtime_tools.shell_policy import repository_owned


OWNED = '[remote "origin"]\n\turl = git@github.com:eunsoogi/codexy.git\n'
OTHER = '[remote "origin"]\n\turl = https://github.com/example/elsewhere.git\n'


def remotes(*urls: str) -> str:
    return "".join(
        f'[remote "remote-{index}"]\n\turl = {url}\n'
        for index, url in enumerate(urls)
    )


class RepositoryIdentityTests(unittest.TestCase):
    def test_missing_config_never_infers_identity_from_directory_name(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve() / "arbitrary-name"
            root.mkdir()
            self.assertIsNone(repository_owned(str(root)))

    def test_exact_remote_matches_and_valid_nonmatch_is_clear(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            dot_git = root / ".git"
            dot_git.mkdir()
            config = dot_git / "config"
            config.write_text(OWNED, encoding="utf-8")
            self.assertIs(repository_owned(str(root)), True)
            config.write_text(OTHER, encoding="utf-8")
            self.assertIs(repository_owned(str(root)), False)

    def test_multiple_remotes_require_unambiguous_consensus(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            dot_git = root / ".git"
            dot_git.mkdir()
            config = dot_git / "config"
            owned = "git@github.com:eunsoogi/codexy.git"
            other = "https://github.com/example/elsewhere.git"
            config.write_text(remotes(owned, other), encoding="utf-8")
            self.assertIsNone(repository_owned(str(root)))
            config.write_text(remotes(owned, owned), encoding="utf-8")
            self.assertIs(repository_owned(str(root)), True)
            config.write_text(remotes(other, other), encoding="utf-8")
            self.assertIs(repository_owned(str(root)), False)

    def test_symlink_and_reparse_git_metadata_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            metadata = root / "metadata"
            metadata.mkdir()
            (metadata / "config").write_text(OWNED, encoding="utf-8")
            dot_git = root / ".git"
            dot_git.symlink_to(metadata, target_is_directory=True)
            self.assertIsNone(repository_owned(str(root)))
            dot_git.unlink()
            dot_git.mkdir()
            (dot_git / "config").write_text(OWNED, encoding="utf-8")
            original = os.lstat

            def reparse(path: os.PathLike[str] | str):
                observed = original(path)
                if Path(path) == dot_git:
                    return SimpleNamespace(
                        st_mode=observed.st_mode,
                        st_dev=observed.st_dev,
                        st_ino=observed.st_ino,
                        st_file_attributes=0x400,
                    )
                return observed

            with mock.patch("os.lstat", side_effect=reparse):
                self.assertIsNone(repository_owned(str(root)))

    def test_malformed_gitfile_and_commondir_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            dot_git = root / ".git"
            dot_git.write_text("not-a-gitdir\n", encoding="utf-8")
            self.assertIsNone(repository_owned(str(root)))
            metadata = root / "metadata" / "worktrees" / "lane"
            metadata.mkdir(parents=True)
            dot_git.write_text("gitdir: metadata/worktrees/lane\n", encoding="utf-8")
            (metadata / "commondir").write_text("bad\nsecond-line\n", encoding="utf-8")
            self.assertIsNone(repository_owned(str(root)))


if __name__ == "__main__":
    unittest.main()
