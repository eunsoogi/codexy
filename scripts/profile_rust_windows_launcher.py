"""Start the Windows profiler workload only after Job assignment."""

from __future__ import annotations

from contextlib import contextmanager
import os
from pathlib import Path
import shutil
import stat
import sys
import tempfile
from typing import Iterator


_RELEASE = b"R"


def retry_readonly_removal(function: object, path: str | Path, error: tuple[object, BaseException, object]) -> None:
    if not isinstance(error[1], PermissionError):
        raise error[1]
    os.chmod(path, stat.S_IWRITE)
    function(path)


class WindowsTempRoot:
    def __init__(
        self,
        original_temp: str,
        original_tmp: str,
        runner_temp: str,
        selected_temp_root: str,
    ) -> None:
        self.original_temp = original_temp
        self.original_tmp = original_tmp
        self.runner_temp = runner_temp
        self.selected_temp_root = selected_temp_root
        self.cleanup = "pending"
        self.cleanup_error = "not-observed"
        self._cleanup_allowed = False

    def allow_cleanup(self) -> None:
        self._cleanup_allowed = True

    def telemetry(self) -> dict[str, str]:
        return {
            "original_temp": self.original_temp,
            "original_tmp": self.original_tmp,
            "runner_temp": self.runner_temp,
            "selected_temp_root": self.selected_temp_root,
            "temp_cleanup": self.cleanup,
            "temp_cleanup_error": self.cleanup_error,
        }


@contextmanager
def isolated_windows_test_root(environment: dict[str, str]) -> Iterator[WindowsTempRoot]:
    runner_temp = environment.get("RUNNER_TEMP")
    if runner_temp is None:
        raise OSError("RUNNER_TEMP is required for the Windows Rust workload")
    runner_root = Path(runner_temp)
    if not runner_root.is_absolute():
        raise OSError("RUNNER_TEMP must be absolute for the Windows Rust workload")
    if not runner_root.is_dir():
        raise OSError("RUNNER_TEMP must name an existing directory for the Windows Rust workload")
    child_root = Path(
        tempfile.mkdtemp(prefix=f"codexy-profile-{os.getpid()}-", dir=runner_root)
    )
    state = WindowsTempRoot(
        original_temp=environment.get("TEMP", "not-observed"),
        original_tmp=environment.get("TMP", "not-observed"),
        runner_temp=str(runner_root),
        selected_temp_root=str(child_root),
    )
    try:
        yield state
    finally:
        if not state._cleanup_allowed:
            state.cleanup = "deferred"
        else:
            try:
                shutil.rmtree(child_root, onerror=retry_readonly_removal)
            except OSError as error:
                state.cleanup = "failed"
                code = getattr(error, "winerror", None) or getattr(error, "errno", None)
                state.cleanup_error = f"{type(error).__name__}:{code or 'not-observed'}"
            else:
                state.cleanup = "removed"


def configure_windows_test_runner(environment: dict[str, str], temp_root: WindowsTempRoot) -> None:
    if "CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUNNER" in environment:
        raise OSError("Windows Rust test runner is already configured")
    runner = Path(__file__).with_name("profile_rust_windows_test_runner.py")
    command = (str(Path(sys.executable)), "-I", "-S", str(runner))
    if any(any(character.isspace() for character in argument) for argument in command):
        raise OSError("Windows Rust test runner command cannot contain whitespace")
    environment["CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUNNER"] = " ".join(command)
    environment["CODEXY_WINDOWS_TEST_TEMP_ROOT"] = temp_root.selected_temp_root


def _raise_after(error: Exception, actions: tuple[object, ...]) -> None:
    cleanup_error: Exception | None = None
    for action in actions:
        try:
            action()
        except Exception as cleanup:
            cleanup_error = cleanup_error or cleanup
    if cleanup_error is not None:
        raise error from cleanup_error
    raise error


def _close_control(process: object) -> None:
    control = process.stdin
    if control is not None:
        control.close()


def _release(process: object) -> None:
    control = process.stdin
    if control is None:
        raise OSError("Windows launcher control pipe is unavailable")
    control.write(_RELEASE)
    control.flush()
    control.close()


def launch_windows_workload(
    job: object, root: Path, capture: object, workload: tuple[str, ...], spawn: object = None,
    environment: dict[str, str] | None = None,
) -> object:
    import subprocess

    command = (sys.executable, "-I", "-S", str(Path(__file__).resolve()), *workload)
    spawn = spawn or subprocess.Popen
    try:
        process = spawn(
            command, cwd=root, stdin=subprocess.PIPE, stdout=capture, stderr=subprocess.STDOUT, env=environment
        )
    except Exception as error:
        _raise_after(error, (job.close,))
    try:
        job.assign(process)
    except Exception as error:
        _raise_after(error, (lambda: _close_control(process), process.kill, process.wait, job.close))
    try:
        _release(process)
    except Exception as error:
        _raise_after(
            error,
            (lambda: _close_control(process), job.terminate_and_wait, process.wait, job.close),
        )
    return process


def main() -> int:
    if sys.stdin.buffer.read(1) != _RELEASE:
        return 64
    import subprocess

    return subprocess.Popen(sys.argv[1:]).wait()


if __name__ == "__main__":
    raise SystemExit(main())
