"""Bounded timing instrumentation for the Windows probe regression."""

from __future__ import annotations

from collections.abc import Callable, Iterator
from contextlib import ExitStack, contextmanager
from time import perf_counter
from typing import Any
from unittest.mock import patch


_MAX_STAGE_SAMPLES = 4
_TIMED_PROCESS_STAGES = (
    ("_terminate_process_tree", "tree_termination"),
    ("_cancel_windows_pipe_threads", "reader_cancel"),
    ("_cancel_synchronous_io", "sync_io_cancel"),
    ("_abort_process_pipe", "pipe_abort"),
    ("_close_process_pipes", "pipe_close"),
    ("_wait_for_exit", "wait_for_exit"),
)


@contextmanager
def capture_process_stages(process: Any) -> Iterator[dict[str, list[float]]]:
    """Time cleanup calls and each communicate attempt without replacing Popen."""
    timings: dict[str, list[float]] = {stage: [] for _, stage in _TIMED_PROCESS_STAGES}
    timings["communicate"] = []

    def measure(stage: str, operation: Callable[..., Any]) -> Callable[..., Any]:
        def measured(*args: Any, **kwargs: Any) -> Any:
            started = perf_counter()
            try:
                return operation(*args, **kwargs)
            finally:
                # The process cleanup path has a small fixed call count; cap each
                # stage so accidental retries cannot grow the diagnostic output.
                samples = timings[stage]
                if len(samples) < _MAX_STAGE_SAMPLES:
                    samples.append(round(perf_counter() - started, 3))

        return measured

    real_popen = process.subprocess.Popen

    def measured_popen(*args: Any, **kwargs: Any) -> Any:
        process_handle = real_popen(*args, **kwargs)
        process_handle.communicate = measure("communicate", process_handle.communicate)
        return process_handle

    with ExitStack() as patches:
        patches.enter_context(
            patch.object(process.subprocess, "Popen", side_effect=measured_popen)
        )
        for function_name, stage_name in _TIMED_PROCESS_STAGES:
            patches.enter_context(
                patch.object(
                    process,
                    function_name,
                    side_effect=measure(stage_name, getattr(process, function_name)),
                )
            )
        yield timings
