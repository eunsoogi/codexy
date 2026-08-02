use std::path::Path;
use std::process::Command;

#[test]
fn windows_profile_uses_a_unique_runner_temp_child_and_cleans_after_capture()
-> Result<(), Box<dyn std::error::Error>> {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/profile-rust-tests");
    let probe = r#"
import json, pathlib, runpy, subprocess, sys, tempfile

script = pathlib.Path(sys.argv[1])
sys.path.insert(0, str(script.parent))
module = runpy.run_path(script)
temp = pathlib.Path(tempfile.mkdtemp())
runner = temp / "runner"
runner.mkdir()
native = temp / "native"
native.mkdir()
seen = {}

class Process:
    pid = 71
    def wait(self, timeout=None): return 0
    def poll(self): return 0

def spawn(*_args, **kwargs):
    seen.update({key: kwargs["env"].get(key) for key in ("TEMP", "TMP", "RUNNER_TEMP")})
    return Process()

module["subprocess"].Popen = spawn
environment = {"TEMP": str(native), "TMP": str(native), "RUNNER_TEMP": str(runner)}
original_environ = module["os"].environ
module["os"].environ = environment
try:
    _output, _elapsed, status, phases = module["run_workload"](temp, 1.0, True)
finally:
    module["os"].environ = original_environ
payload = json.loads(phases["windows-telemetry-json"])
selected = pathlib.Path(seen["TEMP"])
if status or seen["TEMP"] != seen["TMP"] or selected.parent != runner or selected == native:
    raise SystemExit(f"status={status!r} seen={seen!r}")
if selected.exists() or payload.get("temp") != str(native) or payload.get("tmp") != str(native):
    raise SystemExit(f"selected={selected!r} payload={payload!r}")
if payload.get("selected_temp_root") != str(selected) or payload.get("temp_cleanup") != "removed":
    raise SystemExit(f"payload={payload!r}")
"#;
    let output = Command::new("python3")
        .args(["-c", probe])
        .arg(script)
        .output()?;

    assert!(output.status.success(), "{output:?}");
    Ok(())
}

#[test]
fn windows_profile_rejects_invalid_runner_temp_before_launch()
-> Result<(), Box<dyn std::error::Error>> {
    let launcher = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("scripts/profile_rust_windows_launcher.py");
    let probe = r#"
import pathlib, runpy, sys, tempfile

path = pathlib.Path(sys.argv[1])
module = runpy.run_path(path)
root = pathlib.Path(tempfile.mkdtemp())
for environment in ({}, {"RUNNER_TEMP": "relative"}):
    try:
        with module["isolated_windows_temp"](environment):
            raise SystemExit("invalid runner temp launched")
    except OSError:
        pass
saved = module["tempfile"].mkdtemp
module["tempfile"].mkdtemp = lambda **_kwargs: (_ for _ in ()).throw(PermissionError("unwritable"))
try:
    try:
        with module["isolated_windows_temp"]({"RUNNER_TEMP": str(root)}):
            raise SystemExit("unwritable runner temp launched")
    except PermissionError:
        pass
finally:
    module["tempfile"].mkdtemp = saved
"#;
    let output = Command::new("python3")
        .args(["-c", probe])
        .arg(launcher)
        .output()?;

    assert!(output.status.success(), "{output:?}");
    Ok(())
}

#[test]
fn telemetry_ranks_fixture_materializations_by_stable_identity()
-> Result<(), Box<dyn std::error::Error>> {
    let telemetry = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/profile_rust_telemetry.py");
    let probe = r#"
import importlib.util, json, pathlib, sys, tempfile

path = pathlib.Path(sys.argv[1])
spec = importlib.util.spec_from_file_location("telemetry", path)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
metrics = pathlib.Path(tempfile.mkdtemp()) / "fixture-metrics"
metrics.write_text("fixture-materialization\tfull:tests/a.rs:7\t8\t80\nfixture-materialization\tselective:instruction-policy\t2\t20\n")
payload = json.loads(module.telemetry(None, {}, metrics))
expected = [
    {"identity": "full:tests/a.rs:7", "materializations": 1, "files": 8, "bytes": 80},
    {"identity": "selective:instruction-policy", "materializations": 1, "files": 2, "bytes": 20},
]
if payload.get("fixture_materialization_ranked") != expected:
    raise SystemExit(f"payload={payload!r}")
if (payload.get("fixture_materializations"), payload.get("fixture_copied_files"), payload.get("fixture_copied_bytes")) != (2, 10, 100):
    raise SystemExit(f"totals={payload!r}")
"#;
    let output = Command::new("python3")
        .args(["-c", probe])
        .arg(telemetry)
        .output()?;

    assert!(output.status.success(), "{output:?}");
    Ok(())
}
