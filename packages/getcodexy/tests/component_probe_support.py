"""Process-argument helpers for capability probe tests."""

import os
import subprocess
from pathlib import Path
from unittest.mock import patch


def windows_argv(probe, root: Path, platform_os):
    launchers = tuple(
        root / directory / "probe.cmd"
        for directory in ("plain", "codexy&staging", "codexy staging")
    )
    for launcher in launchers:
        launcher.parent.mkdir()
        launcher.write_text("@exit /b 0\r\n", encoding="utf-8")
    python = root / "Python Runtime" / "python.exe"
    with patch.object(probe, "os", platform_os):
        batch = tuple(
            probe._argv(f'"{launcher}" PermissionRequest', root)
            for launcher in launchers
        )
        native = (
            probe._argv("powershell.exe -NoProfile -File hook.ps1", root),
            probe._argv(f'"{python}" hook.py', root),
        )
    executed = (
        tuple(subprocess.run(argv, check=False, timeout=5).returncode for argv in batch)
        if probe.os.name == "nt"
        else ()
    )
    return launchers, batch, native, python, executed
