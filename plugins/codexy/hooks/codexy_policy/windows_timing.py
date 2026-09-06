"""Native Windows safeguards for bounded timing-file writes."""

from __future__ import annotations

import ctypes
import os
import stat
from pathlib import Path

MAX_BYTES = 1024 * 1024
OWNER_RIGHTS_SID = b"\x01\x01\x00\x00\x00\x00\x00\x03\x04\x00\x00\x00"
FILE_ATTRIBUTE_DIRECTORY = 0x10
FILE_ATTRIBUTE_REPARSE_POINT = 0x400


class _SecurityAttributes(ctypes.Structure):
    _fields_ = [
        ("length", ctypes.c_uint32),
        ("descriptor", ctypes.c_void_p),
        ("inherit", ctypes.c_int),
    ]


def open_parents(path: Path) -> list[int]:
    handles, current = [], Path(path.anchor)
    try:
        for part in path.parts[1:-1]:
            current /= part
            handle = _create_file(current, directory=True, create=False)
            try:
                attributes = _attributes(handle)
                if (
                    not attributes & FILE_ATTRIBUTE_DIRECTORY
                    or attributes & FILE_ATTRIBUTE_REPARSE_POINT
                ):
                    raise OSError("timing parent is not a private real directory")
            except OSError:
                close(handle)
                raise
            handles.append(handle)
        return handles
    except OSError:
        for handle in reversed(handles):
            close(handle)
        raise


def open_target(path: Path, create: bool) -> int:
    handle = _create_file(path, directory=False, create=create)
    try:
        if _attributes(handle) & (
            FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT
        ):
            raise OSError("timing target is not a regular file")
        import msvcrt

        descriptor = msvcrt.open_osfhandle(
            handle, os.O_WRONLY | os.O_APPEND | getattr(os, "O_BINARY", 0)
        )
        handle = None
        return descriptor
    finally:
        if handle is not None:
            close(handle)


def _create_file(path: Path, *, directory: bool, create: bool) -> int:
    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel32.CreateFileW.restype = ctypes.c_void_p
    access, flags = (0x80, 0x2200000) if directory else (0x40020000, 0x200000)
    security = None
    attributes = None
    if create and not directory:
        advapi32 = ctypes.WinDLL("advapi32", use_last_error=True)
        security = ctypes.c_void_p()
        if not advapi32.ConvertStringSecurityDescriptorToSecurityDescriptorW(
            ctypes.c_wchar_p("D:P(A;;FA;;;OW)"), 1, ctypes.byref(security), None
        ):
            raise OSError(ctypes.get_last_error(), "security descriptor failed")
        attributes = _SecurityAttributes(
            ctypes.sizeof(_SecurityAttributes), security, 0
        )
    try:
        handle = kernel32.CreateFileW(
            ctypes.c_wchar_p(str(path)),
            access,
            3,
            ctypes.byref(attributes) if attributes is not None else None,
            int(create) or 3,
            flags,
            None,
        )
        if handle == ctypes.c_void_p(-1).value:
            raise OSError(ctypes.get_last_error(), "CreateFileW failed", str(path))
        return int(handle)
    finally:
        if security is not None:
            kernel32.LocalFree(security)


def _attributes(handle: int) -> int:
    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    details = ctypes.create_string_buffer(52)
    if not kernel32.GetFileInformationByHandle(ctypes.c_void_p(handle), details):
        raise OSError(ctypes.get_last_error(), "GetFileInformationByHandle failed")
    return int.from_bytes(details.raw[:4], "little")


def close(handle: int) -> None:
    ctypes.WinDLL("kernel32", use_last_error=True).CloseHandle(ctypes.c_void_p(handle))


def private_writable(details: os.stat_result, descriptor: int | None = None) -> bool:
    if not bool(details.st_mode & stat.S_IWRITE):
        return False
    if getattr(details, "st_file_attributes", 0) & FILE_ATTRIBUTE_REPARSE_POINT:
        return False
    return descriptor is None or _private_acl(descriptor)


def _private_acl(descriptor: int) -> bool:
    advapi32 = ctypes.WinDLL("advapi32", use_last_error=True)
    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    dacl, security = ctypes.c_void_p(), ctypes.c_void_p()
    result = advapi32.GetSecurityInfo(
        ctypes.c_void_p(descriptor),
        1,
        5,
        None,
        None,
        ctypes.byref(dacl),
        None,
        ctypes.byref(security),
    )
    try:
        if result or not dacl.value or not security.value:
            return False
        info = (ctypes.c_uint32 * 3)()
        if (
            not advapi32.GetAclInformation(dacl, info, ctypes.sizeof(info), 2)
            or info[0] != 1
        ):
            return False
        ace = ctypes.c_void_p()
        if not advapi32.GetAce(dacl, 0, ctypes.byref(ace)) or not ace.value:
            return False
        return (
            ctypes.string_at(ace.value, 2) == b"\x00\x00"
            and ctypes.c_uint32.from_address(ace.value + 4).value
            and ctypes.string_at(ace.value + 8, 12) == OWNER_RIGHTS_SID
        )
    finally:
        if security.value:
            kernel32.LocalFree(security)


def try_lock(descriptor: int):
    import msvcrt

    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    handle = ctypes.c_void_p(msvcrt.get_osfhandle(descriptor))
    lock_file = kernel32.LockFile
    lock_file.argtypes = [ctypes.c_void_p] + [ctypes.c_uint32] * 4
    return (kernel32, handle) if lock_file(handle, 0, 0, MAX_BYTES, 0) else False


def unlock(lock) -> None:
    lock[0].UnlockFile(lock[1], 0, 0, MAX_BYTES, 0)
