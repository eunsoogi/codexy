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
if timeout != ("", 1.0, 124) or success != ("", 0.0, 7) or locked[0] or len(jobs) != 2:
    raise SystemExit(f"timeout={timeout!r} success={success!r} locked={locked[0]!r} jobs={jobs!r}")
if [job.deadline for job in jobs] != [11.0, 11.0] or parents[0].waits or parents[1].waits != [None] or not jobs[0].assigned or not jobs[0].terminated or not jobs[1].assigned or jobs[1].terminated:
    raise SystemExit(f"jobs={jobs!r}")
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
parent = "import pathlib,subprocess,sys; sys.stdout.buffer.write(b'first\\r\\n\\xce\\xbc-tail\\r\\n'); sys.stdout.buffer.flush(); sys.stdin.buffer.read(1); p=subprocess.Popen([sys.executable,'-c','import sys; sys.stdin.buffer.read(1)'], stdout=sys.stdout.buffer, stderr=sys.stdout.buffer, close_fds=False); pathlib.Path(sys.argv[1]).write_text(str(p.pid))"

class RaceProcess:
    def __init__(self, capture, writer_pid):
        self.child = subprocess.Popen((sys.executable, "-c", parent, str(writer_pid)), stdin=subprocess.PIPE, stdout=capture, stderr=subprocess.STDOUT, close_fds=False)
        self.pid = self.child.pid
        self._handle = self.child._handle
    def release(self):
        self.child.stdin.write(b"x")
        self.child.stdin.flush()
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
if timeout != ("first\r\nμ-tail\r\n", 0.1, 124) or success[0] != "first\r\nμ-tail\r\n" or not 0 <= success[1] < 1.0 or success[2] != 0:
    raise SystemExit(f"timeout={timeout!r} success={success!r}")
"#;
    let output = Command::new("python")
        .args(["-c", probe])
        .arg(script)
        .output()?;

    assert!(output.status.success(), "{output:?}");
    Ok(())
}
