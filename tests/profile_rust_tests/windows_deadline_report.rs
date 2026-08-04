use std::path::Path;
use std::process::Command;

#[test]
fn deadline_report_keeps_unresolved_long_running_test_context(
) -> Result<(), Box<dyn std::error::Error>> {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/profile-rust-tests");
    let probe = r#"
import io
import pathlib
import runpy
import sys

script = pathlib.Path(sys.argv[1])
sys.path.insert(0, str(script.parent))
module = runpy.run_path(script)
partial = "\n".join((
    "     Running unittests src/lib.rs (target/debug/deps/codexy_runtime-a)",
    "test support::finished ... ok",
    "     Running tests/suites/system_suite.rs (target/debug/deps/suite_system-a)",
    "test system::settled has been running for over 60 seconds",
    "test system::settled ... ok",
    "test system::still_running has been running for over 60 seconds",
    "test system::last_completed ... ok",
))
main_globals = module["main"].__globals__
main_globals["enforce_workflow_contract"] = lambda *_args: None
main_globals["archive_fixture_nested_cargo_build_count"] = lambda _root: 0
main_globals["run_workload"] = lambda *_args: (
    partial,
    1.0,
    124,
    {
        "windows-job-active-zero": "deadline",
        "cargo-root-status": "running",
        "windows-job-pids-json": "[]",
        "windows-job-images-json": "[]",
        "linux-cargo-descendants-json": '[{"command":"target/debug/deps/suite_system-a","pid":641,"ppid":521}]',
        "workload-seconds": 1.0,
        "capture-seconds": 0.0,
        "replay-seconds": 0.0,
    },
)
stream, saved_stdout, saved_argv = io.StringIO(), sys.stdout, sys.argv
sys.stdout = stream
sys.argv = [str(script)]
try:
    status = module["main"]()
finally:
    sys.stdout, sys.argv = saved_stdout, saved_argv
fields = {}
for line in stream.getvalue().splitlines():
    key, *values = line.split("\t")
    fields.setdefault(key, []).append(values)
expected = {
    "deadline-last-running-target": [["suite_system"]],
    "deadline-terminal-target": [["not-observed"]],
    "deadline-next-target-not-started": [["suite_archive"]],
    "deadline-active-test": [["suite_system::system::still_running"]],
    "deadline-last-completed-test": [["suite_system::system::last_completed"]],
    "deadline-linux-cargo-descendants-json": [['[{"command":"target/debug/deps/suite_system-a","pid":641,"ppid":521}]']],
}
if status != 124 or any(fields.get(key) != value for key, value in expected.items()):
    raise SystemExit(f"status={status!r} fields={fields!r}")
"#;
    let output = Command::new("python3")
        .args(["-c", probe])
        .arg(script)
        .output()?;

    assert!(output.status.success(), "{output:?}");
    Ok(())
}
