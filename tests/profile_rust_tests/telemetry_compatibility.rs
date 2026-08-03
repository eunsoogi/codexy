use std::path::Path;
use std::process::Command;

#[test]
fn telemetry_reports_only_observed_values_without_a_repository_root()
-> Result<(), Box<dyn std::error::Error>> {
    let telemetry = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/profile_rust_telemetry.py");
    let probe = r#"
import importlib.util, json, pathlib, sys
path = pathlib.Path(sys.argv[1])
sys.path.insert(0, str(path.parent))
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
if payload["command_interval_ranked"] or payload["command_interval_owner_ranked"]:
    raise SystemExit("legacy telemetry fabricated interval records")
if payload["command_interval_owner_coverage"]["unattributed"] != "not-observed":
    raise SystemExit("legacy telemetry changed owner absence semantics")
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

#[test]
fn runtime_telemetry_is_bounded_and_rejects_invalid_target_or_process_records()
-> Result<(), Box<dyn std::error::Error>> {
    let helper = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("scripts/profile_rust_runtime_telemetry.py");
    let probe = r#"
import importlib.util, json, pathlib, sys

path = pathlib.Path(sys.argv[1])
spec = importlib.util.spec_from_file_location("runtime_telemetry", path)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
receipt = module.receipt(
    ("lib", "suite_all"),
    (("lib", "started", 0.1), ("lib", "ended", 0.3), ("suite_all", "started", 0.4)),
    {"RUST_TEST_THREADS": "3"},
    '[{"pid":1,"image":"C:/Git/bin/git.exe"},{"pid":2,"image":"C:/Python/python.exe"}]',
    '[{"pid":3,"ppid":1,"command":"/bin/bash -c validate"}]',
)
if receipt["test_threads"] != {"state":"configured", "value":"3"}:
    raise SystemExit(receipt)
if receipt["targets"] != [
    {"target":"lib", "state":"completed", "started_seconds":0.1, "ended_seconds":0.3, "elapsed_seconds":0.2},
    {"target":"suite_all", "state":"started", "started_seconds":0.4, "ended_seconds":"not-observed", "elapsed_seconds":"not-observed"},
]:
    raise SystemExit(receipt)
if receipt["process_families"] != {"git":1, "python":1, "shell":1, "validator":0, "other":0}:
    raise SystemExit(receipt)
if module.receipt(("lib",), (), {}, "[]")["test_threads"] != {"state":"default/unobserved", "value":"not-observed"}:
    raise SystemExit("default test-thread state")
if module.process_records('[{"pid":4,"error":"OpenProcess: 5"}]'):
    raise SystemExit("unobserved image became a process family")
observer = module.RuntimeTelemetry(0.0, (), {})
for snapshot in [
    '[{"pid":42,"image":"git"},{"pid":43,"image":"git"}]',
    '[{"pid":42,"image":"python3"}]',
    '[{"pid":44,"image":"python3"},{"pid":45,"image":"python3"}]',
    '[{"pid":42,"image":"python3"}]',
]:
    observer._observe_snapshot(snapshot)
if observer._families != {"git":2, "python":2, "shell":0, "validator":0, "other":0}:
    raise SystemExit(observer._families)
if module.receipt(("lib",), (), {}, [], family_max=observer._families)["process_observation"] != "bounded-snapshot-max-family-concurrency":
    raise SystemExit("ambiguous process observation")
for events, processes in [
    ((("unknown", "started", 0.1),), "[]"),
    ((("lib", "started", 0.1), ("lib", "started", 0.2)), "[]"),
    ((("lib", "started", 0.1),), '[{"pid":"bad","image":"git"}]'),
    ((("lib", "started", 0.1),), '[{"pid":1,"image":"git"},{"pid":1,"image":"git"}]'),
    ((("lib", "started", 0.1),), '[{"pid":1,"image":"git"},{"pid":1,"image":"python3"}]'),
    ((("lib", "started", 0.1),), '[{"pid":1,"image":"git","extra":true}]'),
]:
    try:
        module.receipt(("lib",), events, {}, processes, "not-applicable")
    except ValueError:
        continue
    raise SystemExit(f"accepted invalid record: {events!r} {processes!r}")
"#;
    let output = Command::new("python3")
        .args(["-c", probe])
        .arg(helper)
        .output()?;
    assert!(output.status.success(), "{output:?}");
    Ok(())
}

#[test]
fn profiler_ranks_bounded_command_waits_without_sensitive_command_data()
-> Result<(), Box<dyn std::error::Error>> {
    let helper = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/profile_rust_telemetry.py");
    let probe = r#"
import importlib.util, json, pathlib, sys, tempfile

path = pathlib.Path(sys.argv[1])
sys.path.insert(0, str(path.parent))
spec = importlib.util.spec_from_file_location("profile_telemetry", path)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
metrics = pathlib.Path(tempfile.mkdtemp())
record = metrics / "command-1.metrics"
record.write_text(
    "command-wait\tv1\tfixture-command.output.unattributed:python\tpython\t1\t0.250000\n"
    "command-wait\tv1\tfixture-command.output.unattributed:python\tpython\t1\t0.125000\n"
    "command-wait\tv1\tmcp-client.response\tother\t1\t0.500000\n"
)
payload = json.loads(module.telemetry(None, {}, None, command_metrics_path=metrics))
if payload["command_wait_ranked"] != [
    {"key":"mcp-client.response", "family":"other", "count":1, "cumulative_wait_seconds":0.5},
]:
    raise SystemExit(payload)
if payload["command_wait_unattributed"] != {"count":2, "cumulative_wait_seconds":0.375}:
    raise SystemExit(payload)
if any("/" in json.dumps(record) or "secret" in json.dumps(record) for record in payload["command_wait_ranked"]):
    raise SystemExit("sensitive command data leaked")
for line in [
    "command-wait\tv2\tmcp-client.response\tother\t1\t0.1\n",
    "command-wait\tv1\tmcp-client.response\tother\t2\t0.1\n",
    "command-wait\tv1\tmcp-client.response\tpython\t1\t0.1\n",
    "command-wait\tv1\tfixture-command.output.unattributed:python\tpython\t1\tnan\n",
    "command-wait\tv1\tmcp-client.response\tother\t1\t0.1\textra\n",
]:
    record.write_text(line)
    try:
        module.telemetry(None, {}, None, command_metrics_path=metrics)
    except ValueError:
        continue
    raise SystemExit(f"accepted invalid command metric: {line!r}")
record.unlink()
if json.loads(module.telemetry(None, {}, None, command_metrics_path=metrics))["command_wait_ranked"]:
    raise SystemExit("profiling-disabled metrics were invented")
"#;
    let output = Command::new("python3")
        .args(["-c", probe])
        .arg(helper)
        .output()?;
    assert!(output.status.success(), "{output:?}");
    Ok(())
}
