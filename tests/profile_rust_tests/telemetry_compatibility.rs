use std::path::Path;
use std::process::Command;

#[test]
fn telemetry_reports_only_observed_values_without_a_repository_root()
-> Result<(), Box<dyn std::error::Error>> {
    let telemetry = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/profile_rust_telemetry.py");
    let probe = r#"
import importlib.util, json, pathlib, sys
path = pathlib.Path(sys.argv[1])
spec = importlib.util.spec_from_file_location("telemetry", path)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
payload = json.loads(module.telemetry(None, {"TEMP":"T:/temp", "RUST_TEST_THREADS":"2"}, None))
expected = {
    "temp": "T:/temp",
    "tmp": "not-observed",
    "runner_temp": "not-observed",
    "workspace": "not-observed",
    "target": "not-observed",
    "rust_test_threads": "2",
    "fixture_materializations": 0,
    "fixture_copied_files": 0,
    "fixture_copied_bytes": 0,
}
if any(payload.get(key) != value for key, value in expected.items()):
    raise SystemExit(f"payload={payload!r}")
for key in ("logical_cpus", "available_parallelism"):
    if payload.get(key) != "not-observed" and not isinstance(payload.get(key), int):
        raise SystemExit(f"{key}={payload[key]!r}")
"#;
    let output = Command::new("python3")
        .args(["-c", probe])
        .arg(telemetry)
        .output()?;
    assert!(output.status.success(), "{output:?}");
    Ok(())
}

#[test]
fn mocked_workloads_without_telemetry_preserve_existing_output()
-> Result<(), Box<dyn std::error::Error>> {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/profile-rust-tests");
    let probe = r#"
import io, pathlib, runpy, sys
script = pathlib.Path(sys.argv[1])
sys.path.insert(0, str(script.parent))
module = runpy.run_path(script)
main = module["main"].__globals__
main["enforce_workflow_contract"] = lambda *_args: None
main["archive_fixture_nested_cargo_build_count"] = lambda _root: 0
main["observed_test_outcomes"] = lambda _output: {"ok": 1802, "FAILED": 0, "ignored": 0}
main["run_workload"] = lambda *_args: (
    "test result: ok. 1802 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out",
    1.0,
    0,
    {"windows-job-active-zero":"completed", "cargo-root-status":"0", "windows-job-pids-json":"[]", "windows-job-images-json":"[]", "linux-cargo-descendants-json":"not-applicable", "workload-seconds":0.6, "capture-seconds":0.2, "replay-seconds":0.1},
)
stream, saved_stdout, saved_argv = io.StringIO(), sys.stdout, sys.argv
sys.stdout, sys.argv = stream, [str(script)]
try:
    status = module["main"]()
finally:
    sys.stdout, sys.argv = saved_stdout, saved_argv
output = stream.getvalue()
if status != 0 or "windows-telemetry-json" in output or "result\tPASS" not in output:
    raise SystemExit(f"status={status!r} output={output!r}")
"#;
    let output = Command::new("python3")
        .args(["-c", probe])
        .arg(script)
        .output()?;
    assert!(output.status.success(), "{output:?}");
    Ok(())
}

#[test]
fn non_windows_profile_emits_bounded_fixture_rank_telemetry()
-> Result<(), Box<dyn std::error::Error>> {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/profile-rust-tests");
    let probe = r#"
import io, pathlib, runpy, sys
script = pathlib.Path(sys.argv[1])
sys.path.insert(0, str(script.parent))
module = runpy.run_path(script)
main = module["main"].__globals__
main["enforce_workflow_contract"] = lambda *_args: None
main["archive_fixture_nested_cargo_build_count"] = lambda _root: 0
main["observed_test_outcomes"] = lambda _output: {"ok": 1802, "FAILED": 0, "ignored": 0}
main["run_workload"] = lambda *_args: (
    "test result: ok. 1802 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out",
    1.0, 0,
    {"windows-job-active-zero":"not-applicable", "cargo-root-status":"0", "windows-job-pids-json":"[]", "windows-job-images-json":"[]", "linux-cargo-descendants-json":"not-applicable", "workload-seconds":0.6, "capture-seconds":0.2, "replay-seconds":0.1, "fixture-telemetry-json": '{"fixture_materialization_ranked":[{"identity":"full:tests/a.rs:7","materializations":1,"files":8,"bytes":80,"duration_seconds":0.2}]}'},
)
stream, saved_stdout, saved_argv = io.StringIO(), sys.stdout, sys.argv
sys.stdout, sys.argv = stream, [str(script)]
try:
    status = module["main"]()
finally:
    sys.stdout, sys.argv = saved_stdout, saved_argv
output = stream.getvalue()
if status != 0 or "fixture-telemetry-json\t" not in output or "windows-telemetry-json" in output:
    raise SystemExit(f"status={status!r} output={output!r}")
"#;
    let output = Command::new("python3")
        .args(["-c", probe])
        .arg(script)
        .output()?;

    assert!(output.status.success(), "{output:?}");
    Ok(())
}
