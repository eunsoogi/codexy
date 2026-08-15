use std::process::Command;

#[test]
fn cargo_keeps_native_temp_while_configuring_a_windows_test_runner()
-> Result<(), Box<dyn std::error::Error>> {
    let profile = codexy_runtime::paths::repository_root().join("scripts/profile_rust_tests.py");
    let probe = r#"
import atexit, contextlib, json, pathlib, runpy, shutil, sys, tempfile

profile = pathlib.Path(sys.argv[1])
sys.path.insert(0, str(profile.parent))
module = runpy.run_path(profile)
work = pathlib.Path(tempfile.mkdtemp())
atexit.register(shutil.rmtree, work, ignore_errors=True)
runner = work / "runner"
runner.mkdir()
native = work / "native"
native.mkdir()
seen = {}

class Cargo:
    pid = 71
    def wait(self, timeout=None):
        if not pathlib.Path(seen["CODEXY_WINDOWS_TEST_TEMP_ROOT"]).is_dir():
            raise SystemExit(f"test root was cleaned before Cargo completed: {seen!r}")
        return 0
    def poll(self): return 0

def spawn(*_args, **kwargs):
    seen.update(kwargs["env"])
    return Cargo()

@contextlib.contextmanager
def receipt_environment(_directory):
    yield work, {"TEMP": str(native), "TMP": str(native), "RUNNER_TEMP": str(runner)}

module["subprocess"].Popen = spawn
module["run_workload"].__globals__["receipt_environment"] = receipt_environment
_output, _elapsed, _status, phases = module["run_workload"](work, 1.0, True)
if seen.get("TEMP") != str(native) or seen.get("TMP") != str(native):
    raise SystemExit(f"cargo linker temp changed: {seen!r}")
runner_command = seen.get("CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUNNER", "")
if "profile_rust_windows_test_runner.py" not in runner_command or " -I -S " not in runner_command:
    raise SystemExit(f"runner={runner_command!r}")
if seen.get("CODEXY_WINDOWS_TEST_TEMP_ROOT") is None:
    raise SystemExit(f"test temp root missing: {seen!r}")
selected = pathlib.Path(seen["CODEXY_WINDOWS_TEST_TEMP_ROOT"])
payload = json.loads(phases["windows-telemetry-json"])
if selected.exists() or payload.get("selected_temp_root") != str(selected) or payload.get("temp_cleanup") != "removed":
    raise SystemExit(f"selected={selected!r} payload={payload!r}")
launcher = runpy.run_path(profile.parent / "profile_rust_windows_launcher.py")
state = launcher["WindowsTempRoot"](str(native), str(native), str(runner), str(runner / "selected"))
configured = {}
launcher["configure_windows_test_runner"](configured, state)
try:
    launcher["configure_windows_test_runner"](configured, state)
except OSError:
    pass
else:
    raise SystemExit("runner configuration overwrote a conflicting runner")
saved = launcher["sys"].executable
launcher["sys"].executable = str(work / "Python With Space")
try:
    try:
        launcher["configure_windows_test_runner"]({}, state)
    except OSError:
        pass
    else:
        raise SystemExit("runner configuration accepted whitespace")
finally:
    launcher["sys"].executable = saved
"#;
    let output = Command::new("python3")
        .args(["-c", probe])
        .arg(profile)
        .output()?;

    assert!(output.status.success(), "{output:?}");
    Ok(())
}

#[test]
fn windows_job_drains_test_root_before_outer_cleanup()
-> Result<(), Box<dyn std::error::Error>> {
    let profile = codexy_runtime::paths::repository_root().join("scripts/profile_rust_tests.py");
    let probe = r#"
import atexit, contextlib, pathlib, runpy, shutil, sys, tempfile, types

profile = pathlib.Path(sys.argv[1])
sys.path.insert(0, str(profile.parent))
module = runpy.run_path(profile)
work = pathlib.Path(tempfile.mkdtemp())
atexit.register(shutil.rmtree, work, ignore_errors=True)
runner = work / "runner"; runner.mkdir()
selected = []

class Cargo:
    pid = 71
    def wait(self, timeout=None): return 0
    def poll(self): return 0

class Job:
    def assign(self, _process): pass
    def diagnostics(self, _process): return {}
    def wait_for_empty_until(self, _deadline): return False
    def terminate_and_wait(self):
        if not pathlib.Path(selected[-1]).is_dir():
            raise SystemExit(f"test root was cleaned before job drain: {selected!r}")
    def close(self): pass

@contextlib.contextmanager
def receipt_environment(_directory):
    yield work, {"TEMP": str(work), "TMP": str(work), "RUNNER_TEMP": str(runner)}

def launch(_job, _root, _capture, _workload, environment=None):
    root = pathlib.Path(environment["CODEXY_WINDOWS_TEST_TEMP_ROOT"])
    if not root.is_dir():
        raise SystemExit(f"test root missing at launch: {environment!r}")
    selected.append(root)
    return Cargo()

module["run_workload"].__globals__["os"] = types.SimpleNamespace(name="nt")
module["run_workload"].__globals__["WindowsJob"] = Job
module["run_workload"].__globals__["receipt_environment"] = receipt_environment
module["run_workload"].__globals__["launch_windows_workload"] = launch
_output, _elapsed, status, phases = module["run_workload"](work, 1.0, True)
if status or phases["windows-job-active-zero"] != "drained" or any(root.exists() for root in selected):
    raise SystemExit(f"status={status!r} phases={phases!r} selected={selected!r}")
"#;
    let output = Command::new("python3")
        .args(["-c", probe])
        .arg(profile)
        .output()?;

    assert!(output.status.success(), "{output:?}");
    Ok(())
}

#[test]
fn windows_test_runner_forwards_arguments_output_exit_and_temp()
-> Result<(), Box<dyn std::error::Error>> {
    let runner = codexy_runtime::paths::repository_root()
        .join("scripts/profile_rust_windows_test_runner.py");
    let probe = r#"
import atexit, json, os, pathlib, shutil, subprocess, sys, tempfile, time

runner = pathlib.Path(sys.argv[1])
work = pathlib.Path(tempfile.mkdtemp())
atexit.register(shutil.rmtree, work, ignore_errors=True)
root = work / "runner"
root.mkdir()
native = work / "native"
native.mkdir()
child = "import json,os,pathlib,subprocess,sys; subprocess.Popen((sys.executable, '-c', \"import os,pathlib,time; time.sleep(.1); pathlib.Path(os.environ['CODEXY_RUNNER_MARKER']).write_text(str(pathlib.Path(os.environ['TEMP']).is_dir()))\")); print(json.dumps({'argv':sys.argv[1:], 'cwd':os.getcwd(), 'temp':os.environ['TEMP'], 'tmp':os.environ['TMP'], 'runner':os.getenv('CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUNNER'), 'root':os.getenv('CODEXY_WINDOWS_TEST_TEMP_ROOT')})); print('stderr-marker', file=sys.stderr); raise SystemExit(7)"
marker = work / "descendant-temp-existed"
environment = os.environ | {"CODEXY_WINDOWS_TEST_TEMP_ROOT": str(root), "TEMP": str(native), "TMP": str(native), "CODEXY_RUNNER_MARKER": str(marker)}
arguments = (sys.executable, runner, sys.executable, "-c", child, "", "spaced argument", 'quote"slash\\\\', "$metachar", "한글")
output = subprocess.run(arguments, text=True, capture_output=True, cwd=work, env=environment)
payload = json.loads(output.stdout)
selected = pathlib.Path(payload["temp"])
if output.returncode != 7 or payload["argv"] != list(arguments[5:]) or payload["tmp"] != str(selected) or pathlib.Path(payload["cwd"]).resolve() != work.resolve() or payload["runner"] is not None or payload["root"] is not None:
    raise SystemExit(f"output={(output.returncode, output.stdout, output.stderr)!r}")
if selected.parent != root or selected == native or not selected.exists() or "stderr-marker" not in output.stderr:
    raise SystemExit(f"selected={selected!r} output={(output.returncode, output.stdout, output.stderr)!r}")
for _ in range(20):
    if marker.exists():
        break
    time.sleep(.05)
if marker.read_text() != "True":
    raise SystemExit(f"descendant temp was removed early: {marker.read_text()!r}")
"#;
    let output = Command::new("python3")
        .args(["-c", probe])
        .arg(runner)
        .output()?;

    assert!(output.status.success(), "{output:?}");
    Ok(())
}
