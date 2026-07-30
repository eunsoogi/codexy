use std::path::Path;
use std::process::Command;

#[test]
fn windows_timeout_job_releases_writer_before_capture_cleanup(
) -> Result<(), Box<dyn std::error::Error>> {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/profile-rust-tests");
    let probe = r#"
import pathlib
import runpy
import json
import subprocess
import sys
import tempfile
import types
import io

locked = [False]
parents = []
jobs = []
mode = ["timeout"]
root_status = [0]
script = pathlib.Path(sys.argv[1])
sys.path.insert(0, str(script.parent))
module = runpy.run_path(script)

class WindowsTemporaryDirectory:
    def __init__(self, *_args, **_kwargs):
        pass
    def __enter__(self):
        self.path = tempfile.mkdtemp(prefix="codexy-profile-")
        return self.path
    def __exit__(self, *_args):
        if locked[0]:
            raise PermissionError(32, "The process cannot access the file", "cargo-output")

class CargoParent:
    pid = 521
    def __init__(self):
        self.waits = []
        locked[0] = mode[0] != "success"
    def wait(self, timeout=None):
        self.waits.append(timeout)
        if root_status[0] is None:
            raise subprocess.TimeoutExpired("cargo", timeout)
        return root_status[0]
    def poll(self):
        return root_status[0]
    def kill(self):
        self.running = False

def spawn(*_args, **_kwargs):
    parent = CargoParent()
    parents.append(parent)
    return parent

class WindowsJob:
    def __init__(self):
        self.assigned = False
        self.terminated = False
        jobs.append(self)
    def assign(self, _process):
        self.assigned = True
    def terminate_and_wait(self):
        self.terminated = True
        locked[0] = False
        root_status[0] = 0
    def wait_for_empty_until(self, deadline):
        self.deadline = deadline
        return mode[0] == "success"
    def diagnostics(self, process):
        payloads = {
            "timeout": ([701], [{"pid": 701, "error": "OpenProcess: 5"}]),
            "running": ([702], [{"pid": 702, "error": "QueryFullProcessImageNameW: 122"}]),
            "nonzero": ([703], [{"pid": 703, "image": "C:/writer.exe"}]),
            "success": ([], []),
        }
        pids, images = payloads[mode[0]]
        return {"cargo-root-status": "running" if process.poll() is None else str(process.poll()), "windows-job-pids-json": json.dumps(pids), "windows-job-images-json": json.dumps(images, sort_keys=True)}
    def close(self):
        pass

module["tempfile"].TemporaryDirectory = WindowsTemporaryDirectory
module["subprocess"].Popen = spawn
module["run_workload"].__globals__["os"] = types.SimpleNamespace(name="nt")
module["run_workload"].__globals__["WindowsJob"] = WindowsJob
module["run_workload"].__globals__["time"] = types.SimpleNamespace(monotonic=lambda: 10.0, perf_counter=lambda: 10.0, sleep=lambda _seconds: None)
timeout = module["run_workload"](None, 1.0)
mode[0], root_status[0] = "running", None
running = module["run_workload"](None, 1.0)
mode[0], root_status[0] = "nonzero", 7
nonzero = module["run_workload"](None, 1.0)
mode[0] = "success"
success = module["run_workload"](None, 1.0)
def observed(result, status, root, pids, images):
    return result[:3] == ("", 1.0 if status == 124 else 0.0, status) and result[3].get("cargo-root-status") == root and result[3].get("windows-job-pids-json") == json.dumps(pids) and result[3].get("windows-job-images-json") == json.dumps(images, sort_keys=True)
if not observed(timeout, 0, "0", [701], [{"pid": 701, "error": "OpenProcess: 5"}]) or not observed(running, 124, "running", [702], [{"pid": 702, "error": "QueryFullProcessImageNameW: 122"}]) or not observed(nonzero, 7, "7", [703], [{"pid": 703, "image": "C:/writer.exe"}]) or not observed(success, 0, "0", [], []) or locked[0] or len(jobs) != 4:
    raise SystemExit(f"timeout={timeout!r} running={running!r} nonzero={nonzero!r} success={success!r} locked={locked[0]!r} jobs={jobs!r}")
if [getattr(job, "deadline", None) for job in jobs] != [10.0, None, 10.0, 10.0] or [parent.waits for parent in parents] != [[1.0]] * 4 or not all(job.assigned for job in jobs) or not all(job.terminated for job in jobs[:3]) or jobs[3].terminated:
    raise SystemExit(f"jobs={jobs!r}")
if [result[3]["windows-job-active-zero"] for result in (timeout, running, nonzero, success)] != ["drained", "deadline", "drained", "completed"]:
    raise SystemExit(f"timeout={timeout!r} running={running!r} nonzero={nonzero!r} success={success!r}")

main_globals = module["main"].__globals__
main_globals["enforce_workflow_contract"] = lambda *_args: None
main_globals["archive_fixture_nested_cargo_build_count"] = lambda _root: 0
main_globals["observed_test_outcomes"] = lambda _output: {"ok": 1802, "FAILED": 0, "ignored": 0}
def fake_workload(_root, _budget):
    return "test result: ok. 1802 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out", 1.0, mode[0], {
        "windows-job-active-zero": "completed" if mode[0] == 0 else "deadline",
        "cargo-root-status": "0" if mode[0] == 0 else "running",
        "windows-job-pids-json": "[]",
        "windows-job-images-json": "[]",
        "workload-seconds": 0.6,
        "capture-seconds": 0.2,
        "replay-seconds": 0.1,
    }
main_globals["run_workload"] = fake_workload
def report(status):
    mode[0] = status
    stream, saved, saved_argv = io.StringIO(), sys.stdout, sys.argv
    sys.stdout = stream
    sys.argv = [str(script)]
    try:
        result = module["main"]()
    finally:
        sys.stdout, sys.argv = saved, saved_argv
    return result, stream.getvalue()
passed, passed_output = report(0)
deadline, deadline_output = report(124)
for output, status, active_zero, result in [(passed_output, 0, "completed", "PASS"), (deadline_output, 124, "deadline", "FAIL")]:
    required = {f"child-status\t{status}", f"windows-job-active-zero\t{active_zero}", f"cargo-root-status\t{'0' if status == 0 else 'running'}", "windows-job-pids-json\t[]", "windows-job-images-json\t[]", "phase-workload-seconds\t0.600", "phase-capture-seconds\t0.200", "phase-replay-seconds\t0.100", "phase-inventory-seconds\t0.000", f"result\t{result}"}
    lines = set(output.splitlines())
    if not required <= lines or not any(line.startswith("phase-accounting-seconds\t") for line in lines):
        raise SystemExit(f"output={output!r}")
if passed != 0 or deadline != 124:
    raise SystemExit(f"passed={passed!r} deadline={deadline!r}")
"#;
    let output = Command::new("python3")
        .args(["-c", probe])
        .arg(script)
        .output()?;

    assert!(output.status.success(), "{output:?}");
    Ok(())
}

#[cfg(windows)]
#[test]
fn windows_timeout_job_owns_a_root_exit_writer_race(
) -> Result<(), Box<dyn std::error::Error>> {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/profile-rust-tests");
    let probe = r#"
import pathlib
import runpy
import json
import subprocess
import sys
import tempfile
import types

script = pathlib.Path(sys.argv[1])
sys.path.insert(0, str(script.parent))
module = runpy.run_path(script)
directory = pathlib.Path(tempfile.mkdtemp())
pid_file = directory / "writer.pid"
waits = []
hold_root, return_status = [False], [0]
parent = "import pathlib,subprocess,sys; sys.stdout.buffer.write(b'first\\r\\n\\xce\\xbc-tail\\r\\n'); sys.stdout.buffer.flush(); sys.stdin.buffer.read(1); p=subprocess.Popen([sys.executable,'-c','import sys; sys.stdin.buffer.read(1)'], stdout=sys.stdout.buffer, stderr=sys.stdout.buffer, close_fds=False); pathlib.Path(sys.argv[1]).write_text(str(p.pid))"

class RaceProcess:
    def __init__(self, capture, writer_pid):
        self.child = subprocess.Popen((sys.executable, "-c", parent, str(writer_pid)), stdin=subprocess.PIPE, stdout=capture, stderr=subprocess.STDOUT, close_fds=False)
        self.pid = self.child.pid
        self._handle = self.child._handle
        self.status, self.done = return_status[0], False
    def release(self):
        if not hold_root[0]:
            self.child.stdin.write(b"x")
            self.child.stdin.flush()
    def wait(self, timeout=None):
        try:
            self.child.wait(timeout)
        except subprocess.TimeoutExpired:
            waits.append((timeout, "timeout"))
            raise subprocess.TimeoutExpired("cargo", timeout)
        self.done = True
        waits.append((timeout, self.status))
        return self.status
    def poll(self):
        return self.status if self.done else self.child.poll()

def spawn(*_args, **kwargs):
    return RaceProcess(kwargs["stdout"], pid_file)

legacy_directory = tempfile.TemporaryDirectory()
legacy_root = pathlib.Path(legacy_directory.name)
legacy_pid = legacy_root / "writer.pid"
legacy_capture = legacy_root / "cargo-output"
with legacy_capture.open("wb", buffering=0) as capture:
    legacy = RaceProcess(capture, legacy_pid)
    legacy.release()
    legacy.child.wait()
taskkill = subprocess.run(("taskkill", "/F", "/T", "/PID", str(legacy.pid)), stdout=subprocess.PIPE, stderr=subprocess.PIPE)
try:
    legacy_capture.unlink()
except PermissionError:
    locked = True
else:
    locked = False
subprocess.run(("taskkill", "/F", "/T", "/PID", legacy_pid.read_text()), check=True)
legacy_directory.cleanup()
if taskkill.returncode != 128 or not locked:
    raise SystemExit(f"legacy returncode={taskkill.returncode!r} locked={locked!r} stderr={taskkill.stderr!r}")

class ReleasingJob(module["WindowsJob"]):
    def assign(self, process):
        super().assign(process)
        process.release()

proxy = types.SimpleNamespace(Popen=spawn, STDOUT=subprocess.STDOUT, TimeoutExpired=subprocess.TimeoutExpired)
module["run_workload"].__globals__["subprocess"] = proxy
module["run_workload"].__globals__["WORKLOAD"] = ("cargo",)
module["run_workload"].__globals__["WindowsJob"] = ReleasingJob
try:
    timeout = module["run_workload"](None, 0.1)
    success = module["run_workload"](None, 1.0)
    hold_root[0] = True
    running = module["run_workload"](None, 0.1)
    hold_root[0], return_status[0] = False, 7
    nonzero = module["run_workload"](None, 0.1)
finally:
    import shutil
    shutil.rmtree(directory)
def observed(result, status, active_zero, root_status):
    output, _elapsed, actual_status, phases = result
    pids, images = json.loads(phases.get("windows-job-pids-json", "null")), json.loads(phases.get("windows-job-images-json", "null"))
    return output == "first\r\nμ-tail\r\n" and actual_status == status and phases.get("windows-job-active-zero") == active_zero and phases.get("cargo-root-status") == root_status and pids and isinstance(images, list) and all(any(image.get("pid") == pid for image in images) for pid in pids) and all(0 <= phases.get(phase, -1) < 10 for phase in ("workload-seconds", "capture-seconds", "replay-seconds"))
if not observed(timeout, 0, "drained", "0") or not observed(success, 0, "drained", "0") or not observed(running, 124, "deadline", "running") or not observed(nonzero, 7, "drained", "7") or not all(0 < timeout <= budget and status == expected for (timeout, status), budget, expected in zip(waits, (.1, 1, .1, .1), (0, 0, "timeout", 7))):
    raise SystemExit(f"timeout={timeout!r} success={success!r} running={running!r} nonzero={nonzero!r} waits={waits!r}")
"#;
    let output = Command::new("python")
        .args(["-c", probe])
        .arg(script)
        .output()?;

    assert!(output.status.success(), "{output:?}");
    Ok(())
}
