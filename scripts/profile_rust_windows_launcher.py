"""Start the Windows profiler workload only after Job assignment."""

from __future__ import annotations

import sys
from pathlib import Path


_RELEASE = b"R"


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
