use std::path::Path;
use std::process::Command;

#[test]
fn windows_timeout_terminates_the_writer_descendant_before_capture_cleanup(
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
        self.running = True
        locked[0] = True
    def wait(self, timeout=None):
        if self.running:
            raise subprocess.TimeoutExpired("cargo", timeout)
        return 0
    def poll(self):
        return None if self.running else 0
    def kill(self):
        self.running = False

def spawn(*_args, **_kwargs):
    parent = CargoParent()
    parents.append(parent)
    return parent

def taskkill(command, **_kwargs):
    expected = ("taskkill", "/F", "/T", "/PID", "521")
    if tuple(command) != expected:
        raise SystemExit(f"unexpected Windows process-tree command: {command!r}")
    parents[0].running = False
    locked[0] = False

module["tempfile"].TemporaryDirectory = WindowsTemporaryDirectory
module["subprocess"].Popen = spawn
module["subprocess"].run = taskkill
module["run_workload"].__globals__["os"] = types.SimpleNamespace(name="nt")
output, _elapsed, status = module["run_workload"](None, 1.0)
if output or status != 124 or locked[0]:
    raise SystemExit(f"output={output!r} status={status!r} locked={locked[0]!r}")
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
fn windows_timeout_releases_a_real_inherited_capture_handle(
) -> Result<(), Box<dyn std::error::Error>> {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/profile-rust-tests");
    let probe = r#"
import os
import pathlib
import runpy
import subprocess
import sys
import tempfile

script = pathlib.Path(sys.argv[1])
sys.path.insert(0, str(script.parent))
module = runpy.run_path(script)
pid_file = pathlib.Path(tempfile.mkdtemp()) / "writer.pid"
os.environ["PROFILE_CAPTURE_WRITER_PID"] = str(pid_file)
child = "import os,pathlib,time; sys.stdout.buffer.write(b'workload-begin\\n'); sys.stdout.buffer.flush(); p=subprocess.Popen([sys.executable,'-c','import time; time.sleep(60)'], stdout=sys.stdout.buffer, stderr=sys.stdout.buffer, close_fds=False); pathlib.Path(os.environ['PROFILE_CAPTURE_WRITER_PID']).write_text(str(p.pid)); time.sleep(60)"
module["run_workload"].__globals__["WORKLOAD"] = (sys.executable, "-c", "import subprocess,sys; " + child)
try:
    output, elapsed, status = module["run_workload"](None, 0.1)
finally:
    if pid_file.exists():
        subprocess.run(("taskkill", "/F", "/T", "/PID", pid_file.read_text()), check=False)
if output != "workload-begin\n" or elapsed != 0.1 or status != 124:
    raise SystemExit(f"output={output!r} elapsed={elapsed!r} status={status!r}")
"#;
    let output = Command::new("python")
        .args(["-c", probe])
        .arg(script)
        .output()?;

    assert!(output.status.success(), "{output:?}");
    Ok(())
}
