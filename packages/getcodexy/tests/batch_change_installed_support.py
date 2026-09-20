"""Support helpers for the installed batch-change integration test."""

from __future__ import annotations

import json
import shutil
import signal
import subprocess
import time
from pathlib import Path
from typing import Mapping


def install_core_component(repository: Path, install_root: Path) -> Path:
    manifest_path = (
        repository
        / "packages/getcodexy/src/codexy_runtime_tools/component-manifest.json"
    )
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    component = next(item for item in manifest["components"] if item["id"] == "core")
    source = repository / component["asset"]["packageRoot"]
    installed = install_root / component["asset"]["packageRoot"]
    shutil.copytree(source, installed)
    plugin_manifest = json.loads(
        (installed / ".codex-plugin/plugin.json").read_text(encoding="utf-8")
    )
    if plugin_manifest.get("name") != component["plugin"]:
        raise AssertionError(
            "installed plugin identity differs from component manifest"
        )
    missing = [
        path
        for path in component["asset"]["requiredPaths"]
        if not (installed / path).is_file()
    ]
    if missing:
        raise AssertionError(
            f"installed component is missing required paths: {missing}"
        )
    guide = (installed / "skills/engineering/SKILL.md").read_text(encoding="utf-8")
    if "batch_change.py" not in guide:
        raise AssertionError("installed engineering guide omits the batch entrypoint")
    if (
        "skills/engineering/references/batch-changes.md"
        not in component["asset"]["requiredPaths"]
    ):
        raise AssertionError(
            "component manifest omits the installed batch guide reference"
        )
    return installed


def run_boundary_interruption(
    python: str,
    installed_scripts: Path,
    workspace: Path,
    temporary_root: Path,
    result_path: Path,
    item_id: str,
    environment: Mapping[str, str],
) -> dict[str, object]:
    ready = workspace / "replacement-ready"
    ready.unlink(missing_ok=True)
    driver = temporary_root / "installed-boundary-driver.py"
    driver.write_text(
        """import json, signal, sys, time
from pathlib import Path
from threading import Event
sys.path.insert(0, sys.argv[1])
from batch_change_apply.workflow import apply_from_path
cancel = Event()
def handle(signum, frame):
    del signum, frame
    cancel.set()
signal.signal(signal.SIGINT, handle)
ready = Path(sys.argv[5])
def before_replace(path):
    ready.write_text(str(path), encoding='utf-8')
    while not cancel.is_set():
        time.sleep(0.01)
result = apply_from_path(
    Path(sys.argv[2]), sys.argv[3], selected_ids=[sys.argv[4]],
    cancellation_event=cancel, before_replace=before_replace,
    entrypoint=str(Path(sys.argv[1]) / 'batch_change.py'),
)
print(json.dumps(result), flush=True)
""",
        encoding="utf-8",
    )
    process = subprocess.Popen(
        [
            python,
            str(driver),
            str(installed_scripts),
            str(workspace),
            str(result_path),
            item_id,
            str(ready),
        ],
        cwd=workspace,
        env=dict(environment),
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    try:
        deadline = time.monotonic() + 8
        while time.monotonic() < deadline and not ready.is_file():
            if process.poll() is not None:
                break
            time.sleep(0.005)
        if not ready.is_file():
            stdout, stderr = process.communicate(timeout=5)
            raise AssertionError(
                f"replacement boundary was not reached: {stderr}{stdout}"
            )
        process.send_signal(signal.SIGINT)
        stdout, stderr = process.communicate(timeout=15)
    finally:
        if process.poll() is None:
            process.kill()
            process.communicate(timeout=5)
    if process.returncode != 0 or stderr:
        raise AssertionError(f"installed boundary driver failed: {stderr}{stdout}")
    return json.loads(stdout)
