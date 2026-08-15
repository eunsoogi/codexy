"""Windows Job Object ownership for the profiler workload."""

from __future__ import annotations

import ctypes
import json
import time
from ctypes import wintypes


_ASSOCIATE_COMPLETION_PORT = 7
_BASIC_PROCESS_ID_LIST = 3
_EXTENDED_LIMIT_INFORMATION = 9
_ACTIVE_PROCESS_ZERO = 4
_KILL_ON_JOB_CLOSE = 0x00002000
_CLEANUP_SECONDS = 10.0
_WAIT_TIMEOUT = 258
_MORE_DATA = 234
_PROCESS_QUERY_LIMITED_INFORMATION = 0x1000
_INVALID_HANDLE_VALUE = ctypes.c_void_p(-1).value


class _IoCounters(ctypes.Structure):
    _fields_ = [
        (name, ctypes.c_ulonglong)
        for name in (
            "ReadOperationCount",
            "WriteOperationCount",
            "OtherOperationCount",
            "ReadTransferCount",
            "WriteTransferCount",
            "OtherTransferCount",
        )
    ]


class _BasicLimitInformation(ctypes.Structure):
    _fields_ = [
        ("PerProcessUserTimeLimit", ctypes.c_longlong),
        ("PerJobUserTimeLimit", ctypes.c_longlong),
        ("LimitFlags", wintypes.DWORD),
        ("MinimumWorkingSetSize", ctypes.c_size_t),
        ("MaximumWorkingSetSize", ctypes.c_size_t),
        ("ActiveProcessLimit", wintypes.DWORD),
        ("Affinity", ctypes.c_size_t),
        ("PriorityClass", wintypes.DWORD),
        ("SchedulingClass", wintypes.DWORD),
    ]


class _ExtendedLimitInformation(ctypes.Structure):
    _fields_ = [
        ("BasicLimitInformation", _BasicLimitInformation),
        ("IoInfo", _IoCounters),
        ("ProcessMemoryLimit", ctypes.c_size_t),
        ("JobMemoryLimit", ctypes.c_size_t),
        ("PeakProcessMemoryUsed", ctypes.c_size_t),
        ("PeakJobMemoryUsed", ctypes.c_size_t),
    ]


class _AssociateCompletionPort(ctypes.Structure):
    _fields_ = [("CompletionKey", wintypes.LPVOID), ("CompletionPort", wintypes.HANDLE)]


class _BasicProcessIdList(ctypes.Structure):
    _fields_ = [
        ("NumberOfAssignedProcesses", wintypes.DWORD),
        ("NumberOfProcessIdsInList", wintypes.DWORD),
        ("ProcessIdList", ctypes.c_size_t * 1),
    ]


def _kernel32() -> ctypes.WinDLL:
    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    handle = wintypes.HANDLE
    pointer = wintypes.LPVOID
    functions = (
        (
            kernel32.CreateIoCompletionPort,
            (handle, handle, ctypes.c_size_t, wintypes.DWORD),
            handle,
        ),
        (kernel32.CreateJobObjectW, (pointer, wintypes.LPCWSTR), handle),
        (
            kernel32.SetInformationJobObject,
            (handle, ctypes.c_int, pointer, wintypes.DWORD),
            wintypes.BOOL,
        ),
        (kernel32.AssignProcessToJobObject, (handle, handle), wintypes.BOOL),
        (kernel32.TerminateJobObject, (handle, wintypes.UINT), wintypes.BOOL),
        (
            kernel32.GetQueuedCompletionStatus,
            (
                handle,
                ctypes.POINTER(wintypes.DWORD),
                ctypes.POINTER(ctypes.c_size_t),
                ctypes.POINTER(pointer),
                wintypes.DWORD,
            ),
            wintypes.BOOL,
        ),
        (
            kernel32.QueryInformationJobObject,
            (
                handle,
                ctypes.c_int,
                pointer,
                wintypes.DWORD,
                ctypes.POINTER(wintypes.DWORD),
            ),
            wintypes.BOOL,
        ),
        (kernel32.OpenProcess, (wintypes.DWORD, wintypes.BOOL, wintypes.DWORD), handle),
        (
            kernel32.QueryFullProcessImageNameW,
            (handle, wintypes.DWORD, wintypes.LPWSTR, ctypes.POINTER(wintypes.DWORD)),
            wintypes.BOOL,
        ),
        (kernel32.CloseHandle, (handle,), wintypes.BOOL),
    )
    for function, argtypes, restype in functions:
        function.argtypes, function.restype = argtypes, restype
    return kernel32


def _require(value: object, action: str) -> None:
    if not value:
        raise OSError(ctypes.get_last_error(), action)


class WindowsJob:
    def __init__(self) -> None:
        kernel32 = _kernel32()
        self._kernel32 = kernel32
        self._port = kernel32.CreateIoCompletionPort(_INVALID_HANDLE_VALUE, None, 0, 1)
        _require(self._port, "CreateIoCompletionPort")
        self._job = kernel32.CreateJobObjectW(None, None)
        _require(self._job, "CreateJobObjectW")
        association = _AssociateCompletionPort(None, self._port)
        _require(
            kernel32.SetInformationJobObject(
                self._job,
                _ASSOCIATE_COMPLETION_PORT,
                ctypes.byref(association),
                ctypes.sizeof(association),
            ),
            "SetInformationJobObject(completion port)",
        )
        limits = _ExtendedLimitInformation()
        limits.BasicLimitInformation.LimitFlags = _KILL_ON_JOB_CLOSE
        _require(
            kernel32.SetInformationJobObject(
                self._job,
                _EXTENDED_LIMIT_INFORMATION,
                ctypes.byref(limits),
                ctypes.sizeof(limits),
            ),
            "SetInformationJobObject(kill on close)",
        )

    def assign(self, process: object) -> None:
        _require(
            self._kernel32.AssignProcessToJobObject(self._job, process._handle),
            "AssignProcessToJobObject",
        )

    def terminate_and_wait(self) -> None:
        _require(
            self._kernel32.TerminateJobObject(self._job, 124), "TerminateJobObject"
        )
        if not self.wait_for_empty_until(time.monotonic() + _CLEANUP_SECONDS):
            raise TimeoutError(
                "Job Object did not reach active-process-zero after termination"
            )

    def wait_for_empty_until(self, deadline: float) -> bool:
        transferred = wintypes.DWORD()
        key = ctypes.c_size_t()
        overlapped = wintypes.LPVOID()
        while True:
            remaining = max(0, int((deadline - time.monotonic()) * 1000))
            completed = self._kernel32.GetQueuedCompletionStatus(
                self._port,
                ctypes.byref(transferred),
                ctypes.byref(key),
                ctypes.byref(overlapped),
                remaining,
            )
            if not completed:
                error = ctypes.get_last_error()
                if error == _WAIT_TIMEOUT:
                    return False
                raise OSError(error, "GetQueuedCompletionStatus")
            if transferred.value == _ACTIVE_PROCESS_ZERO:
                return True

    def diagnostics(self, process: object) -> dict[str, str]:
        root_status = process.poll()
        pids = self._process_ids()
        images = [self._process_image(pid) for pid in pids]
        return {
            "cargo-root-status": "running" if root_status is None else str(root_status),
            "windows-job-pids-json": json.dumps(pids),
            "windows-job-images-json": json.dumps(images, sort_keys=True),
        }

    def _process_ids(self) -> list[int]:
        capacity = 16
        offset = _BasicProcessIdList.ProcessIdList.offset
        while True:
            size = offset + capacity * ctypes.sizeof(ctypes.c_size_t)
            buffer = ctypes.create_string_buffer(size)
            returned = wintypes.DWORD()
            if self._kernel32.QueryInformationJobObject(
                self._job, _BASIC_PROCESS_ID_LIST, buffer, size, ctypes.byref(returned)
            ):
                header = _BasicProcessIdList.from_buffer(buffer)
                count = min(header.NumberOfProcessIdsInList, capacity)
                values = (ctypes.c_size_t * count).from_buffer(buffer, offset)
                return [int(pid) for pid in values]
            error = ctypes.get_last_error()
            if error != _MORE_DATA:
                raise OSError(error, "QueryInformationJobObject(process ids)")
            capacity = max(
                capacity * 2,
                (returned.value - offset + ctypes.sizeof(ctypes.c_size_t) - 1)
                // ctypes.sizeof(ctypes.c_size_t),
            )

    def _process_image(self, pid: int) -> dict[str, int | str]:
        process = self._kernel32.OpenProcess(
            _PROCESS_QUERY_LIMITED_INFORMATION, False, pid
        )
        if not process:
            return {"pid": pid, "error": f"OpenProcess: {ctypes.get_last_error()}"}
        result: dict[str, int | str] = {"pid": pid}
        try:
            length = wintypes.DWORD(32768)
            buffer = ctypes.create_unicode_buffer(length.value)
            if self._kernel32.QueryFullProcessImageNameW(
                process, 0, buffer, ctypes.byref(length)
            ):
                result["image"] = buffer.value
            else:
                result["error"] = (
                    f"QueryFullProcessImageNameW: {ctypes.get_last_error()}"
                )
        finally:
            if not self._kernel32.CloseHandle(process):
                result.setdefault(
                    "error", f"CloseHandle(process): {ctypes.get_last_error()}"
                )
        return result

    def close(self) -> None:
        _require(self._kernel32.CloseHandle(self._job), "CloseHandle(job)")
        _require(self._kernel32.CloseHandle(self._port), "CloseHandle(completion port)")
