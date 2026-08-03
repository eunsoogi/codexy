use std::path::Path;
use std::process::Command;

#[test]
fn windows_temp_cleanup_failure_is_reported_without_masking_the_workload()
-> Result<(), Box<dyn std::error::Error>> {
    let launcher = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("scripts/profile_rust_windows_launcher.py");
    let probe = r#"
import pathlib, runpy, sys, tempfile
module = runpy.run_path(pathlib.Path(sys.argv[1]))
runner = pathlib.Path(tempfile.mkdtemp())
state = None
saved = module["shutil"].rmtree
module["shutil"].rmtree = lambda _path, **_kwargs: (_ for _ in ()).throw(PermissionError(5, "locked"))
try:
    with module["isolated_windows_test_root"]({"RUNNER_TEMP": str(runner)}) as state:
        state.allow_cleanup()
finally:
    module["shutil"].rmtree = saved
payload = state.telemetry()
if payload.get("temp_cleanup") != "failed" or payload.get("temp_cleanup_error") != "PermissionError:5":
    raise SystemExit(f"payload={payload!r}")
"#;
    let output = Command::new("python3").args(["-c", probe]).arg(launcher).output()?;
    assert!(output.status.success(), "{output:?}");
    Ok(())
}

#[test]
fn windows_profile_fails_closed_when_post_drain_cleanup_is_locked()
-> Result<(), Box<dyn std::error::Error>> {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/profile-rust-tests");
    let launcher = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("scripts/profile_rust_windows_launcher.py");
    let probe = r#"
import contextlib, pathlib, runpy, sys, tempfile
script, launcher_path = map(pathlib.Path, sys.argv[1:])
sys.path.insert(0, str(script.parent))
module = runpy.run_path(script); launcher = runpy.run_path(launcher_path)
work = pathlib.Path(tempfile.mkdtemp()); runner = work / "runner"; runner.mkdir(); native = work / "native"; native.mkdir()
class Process:
    pid = 7
    def wait(self, timeout=None): return 0
    def poll(self): return 0
@contextlib.contextmanager
def receipts(_directory): yield work, {"TEMP": str(native), "TMP": str(native), "RUNNER_TEMP": str(runner)}
@contextlib.contextmanager
def locked_root(environment):
    original = launcher["shutil"].rmtree
    launcher["shutil"].rmtree = lambda _path, **_kwargs: (_ for _ in ()).throw(PermissionError(5, "locked"))
    try:
        with launcher["isolated_windows_test_root"](environment) as state: yield state
    finally: launcher["shutil"].rmtree = original
module["subprocess"].Popen = lambda *_args, **_kwargs: Process()
module["run_workload"].__globals__["receipt_environment"] = receipts
module["run_workload"].__globals__["isolated_windows_test_root"] = locked_root
_output, _elapsed, status, phases = module["run_workload"](work, 1.0, True)
if status != 1 or '"temp_cleanup": "failed"' not in phases["windows-temp-cleanup-receipt-json"]:
    raise SystemExit(f"status={status!r} phases={phases!r}")
"#;
    let output = Command::new("python3").args(["-c", probe]).arg(script).arg(launcher).output()?;
    assert!(output.status.success(), "{output:?}");
    Ok(())
}

#[test]
fn windows_cleanup_normalizes_one_readonly_entry_before_fail_closed_reporting()
-> Result<(), Box<dyn std::error::Error>> {
    let launcher = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("scripts/profile_rust_windows_launcher.py");
    let probe = r#"
import pathlib, runpy, stat, sys, tempfile
module = runpy.run_path(pathlib.Path(sys.argv[1])); path = pathlib.Path(tempfile.mkdtemp()) / "readonly"; calls = []
saved = module["os"].chmod; module["os"].chmod = lambda target, mode: calls.append((target, mode))
try: module["retry_readonly_removal"](lambda target: calls.append((target, "retry")), path, (PermissionError, PermissionError(5, "readonly"), None))
finally: module["os"].chmod = saved
if calls != [(path, stat.S_IWRITE), (path, "retry")]: raise SystemExit(f"calls={calls!r}")
"#;
    let output = Command::new("python3").args(["-c", probe]).arg(launcher).output()?;
    assert!(output.status.success(), "{output:?}");
    Ok(())
}
