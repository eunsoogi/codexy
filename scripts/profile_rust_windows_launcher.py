"""Start the Windows profiler workload only after Job assignment."""

from __future__ import annotations

from contextlib import contextmanager
import os
from pathlib import Path
import shutil
import sys
import tempfile
from typing import Iterator


_RELEASE = b"R"


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

    def telemetry(self) -> dict[str, str]:
        return {
            "original_temp": self.original_temp,
            "original_tmp": self.original_tmp,
            "runner_temp": self.runner_temp,
            "selected_temp_root": self.selected_temp_root,
            "temp_cleanup": self.cleanup,
        }


@contextmanager
def isolated_windows_temp(environment: dict[str, str]) -> Iterator[WindowsTempRoot]:
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
    environment["TEMP"] = state.selected_temp_root
    environment["TMP"] = state.selected_temp_root
    try:
        yield state
    finally:
        try:
            shutil.rmtree(child_root)
        except OSError:
            state.cleanup = "failed"
            raise
        else:
            state.cleanup = "removed"


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
