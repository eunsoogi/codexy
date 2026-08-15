"""Windows Job Object process diagnostics."""

from __future__ import annotations

import ctypes
import json
from ctypes import wintypes

_BASIC_PROCESS_ID_LIST = 3
_MORE_DATA = 234
_PROCESS_QUERY_LIMITED_INFORMATION = 0x1000


def diagnostics(
    kernel32: ctypes.WinDLL,
    job: object,
    process: object,
    process_ids: type[ctypes.Structure],
) -> dict[str, str]:
    root_status = process.poll()
    pids = _process_ids(kernel32, job, process_ids)
    images = [_process_image(kernel32, pid) for pid in pids]
    return {
        "cargo-root-status": "running" if root_status is None else str(root_status),
        "windows-job-pids-json": json.dumps(pids),
        "windows-job-images-json": json.dumps(images, sort_keys=True),
    }


def _process_ids(
    kernel32: ctypes.WinDLL, job: object, process_ids: type[ctypes.Structure]
) -> list[int]:
    capacity, offset = 16, process_ids.ProcessIdList.offset
    while True:
        size = offset + capacity * ctypes.sizeof(ctypes.c_size_t)
        buffer, returned = ctypes.create_string_buffer(size), wintypes.DWORD()
        if kernel32.QueryInformationJobObject(
            job, _BASIC_PROCESS_ID_LIST, buffer, size, ctypes.byref(returned)
        ):
            header = process_ids.from_buffer(buffer)
            count = min(header.NumberOfProcessIdsInList, capacity)
            return [
                int(pid)
                for pid in (ctypes.c_size_t * count).from_buffer(buffer, offset)
            ]
        error = ctypes.get_last_error()
        if error != _MORE_DATA:
            raise OSError(error, "QueryInformationJobObject(process ids)")
        capacity = max(
            capacity * 2,
            (returned.value - offset + ctypes.sizeof(ctypes.c_size_t) - 1)
            // ctypes.sizeof(ctypes.c_size_t),
        )


def _process_image(kernel32: ctypes.WinDLL, pid: int) -> dict[str, int | str]:
    process = kernel32.OpenProcess(_PROCESS_QUERY_LIMITED_INFORMATION, False, pid)
    if not process:
        return {"pid": pid, "error": f"OpenProcess: {ctypes.get_last_error()}"}
    result: dict[str, int | str] = {"pid": pid}
    try:
        length = wintypes.DWORD(32768)
        buffer = ctypes.create_unicode_buffer(length.value)
        if kernel32.QueryFullProcessImageNameW(
            process, 0, buffer, ctypes.byref(length)
        ):
            result["image"] = buffer.value
        else:
            result["error"] = f"QueryFullProcessImageNameW: {ctypes.get_last_error()}"
    finally:
        if not kernel32.CloseHandle(process):
            result.setdefault(
                "error", f"CloseHandle(process): {ctypes.get_last_error()}"
            )
    return result
