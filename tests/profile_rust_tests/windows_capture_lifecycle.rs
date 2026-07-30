use std::path::Path;
use std::process::Command;

#[test]
fn windows_timeout_job_releases_writer_before_capture_cleanup(
) -> Result<(), Box<dyn std::error::Error>> {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/profile-rust-tests");
    let probe = r#"
import pathlib
import runpy
import subprocess
import sys
import tempfile
import types
import io

locked = [False]
parents = []
jobs = []
mode = ["timeout"]
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
        locked[0] = mode[0] == "timeout"
    def wait(self, timeout=None):
        self.waits.append(timeout)
        return 7
    def poll(self):
        return 0
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
    def wait_for_empty_until(self, deadline):
        self.deadline = deadline
        return mode[0] == "success"
    def close(self):
        pass

module["tempfile"].TemporaryDirectory = WindowsTemporaryDirectory
module["subprocess"].Popen = spawn
module["run_workload"].__globals__["os"] = types.SimpleNamespace(name="nt")
module["run_workload"].__globals__["WindowsJob"] = WindowsJob
module["run_workload"].__globals__["time"] = types.SimpleNamespace(monotonic=lambda: 10.0, perf_counter=lambda: 10.0, sleep=lambda _seconds: None)
timeout = module["run_workload"](None, 1.0)
mode[0] = "success"
success = module["run_workload"](None, 1.0)
if timeout[:3] != ("", 1.0, 124) or success[:3] != ("", 0.0, 7) or locked[0] or len(jobs) != 2:
    raise SystemExit(f"timeout={timeout!r} success={success!r} locked={locked[0]!r} jobs={jobs!r}")
if [job.deadline for job in jobs] != [11.0, 11.0] or parents[0].waits or parents[1].waits != [None] or not jobs[0].assigned or not jobs[0].terminated or not jobs[1].assigned or jobs[1].terminated:
    raise SystemExit(f"jobs={jobs!r}")
if timeout[3]["windows-job-active-zero"] != "deadline" or success[3]["windows-job-active-zero"] != "completed":
    raise SystemExit(f"timeout={timeout!r} success={success!r}")

main_globals = module["main"].__globals__
main_globals["enforce_workflow_contract"] = lambda *_args: None
main_globals["archive_fixture_nested_cargo_build_count"] = lambda _root: 0
main_globals["observed_test_outcomes"] = lambda _output: {"ok": 1802, "FAILED": 0, "ignored": 0}
def fake_workload(_root, _budget):
    return "test result: ok. 1802 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out", 1.0, mode[0], {
        "windows-job-active-zero": "completed" if mode[0] == 0 else "deadline",
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
    required = {f"child-status\t{status}", f"windows-job-active-zero\t{active_zero}", "phase-workload-seconds\t0.600", "phase-capture-seconds\t0.200", "phase-replay-seconds\t0.100", "phase-inventory-seconds\t0.000", f"result\t{result}"}
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
parent = "import pathlib,subprocess,sys; sys.stdout.buffer.write(b'first\\r\\n\\xce\\xbc-tail\\r\\n'); sys.stdout.buffer.flush(); sys.stdin.buffer.read(1); p=subprocess.Popen([sys.executable,'-c','import sys; sys.stdin.buffer.read(1)'], stdout=sys.stdout.buffer, stderr=sys.stdout.buffer, close_fds=False); pathlib.Path(sys.argv[1]).write_text(str(p.pid))"

class RaceProcess:
    def __init__(self, capture, writer_pid):
        self.child = subprocess.Popen((sys.executable, "-c", parent, str(writer_pid)), stdin=subprocess.PIPE, stdout=capture, stderr=subprocess.STDOUT, close_fds=False)
        self.pid = self.child.pid
        self._handle = self.child._handle
    def release(self):
        self.child.stdin.write(b"x")
        self.child.stdin.flush()
    def wait(self, timeout=None):
        status = self.child.wait(timeout)
        waits.append((timeout, status))
        return status
    def poll(self):
        return None

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
    parent = "import subprocess,sys; sys.stdout.buffer.write(b'first\\r\\n\\xce\\xbc-tail\\r\\n'); sys.stdout.buffer.flush(); sys.stdin.buffer.read(1); subprocess.Popen([sys.executable,'-c','pass'], stdout=sys.stdout.buffer, stderr=sys.stdout.buffer, close_fds=False)"
    success = module["run_workload"](None, 1.0)
finally:
    import shutil
    shutil.rmtree(directory)
def observed(result, status, active_zero):
    output, _elapsed, actual_status, phases = result
    return output == "first\r\nμ-tail\r\n" and actual_status == status and phases.get("windows-job-active-zero") == active_zero and all(0 <= phases.get(phase, -1) < 10 for phase in ("workload-seconds", "capture-seconds", "replay-seconds"))
if not observed(timeout, 124, "deadline") or not observed(success, 0, "completed") or not 0 <= success[1] < 1.0 or waits != [(None, 0)]:
    raise SystemExit(f"timeout={timeout!r} success={success!r} waits={waits!r}")
"#;
    let output = Command::new("python")
        .args(["-c", probe])
        .arg(script)
        .output()?;

    assert!(output.status.success(), "{output:?}");
    Ok(())
}
