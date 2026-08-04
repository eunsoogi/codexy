use std::path::Path;
use std::process::Command;

#[test]
fn archive_inspector_receipts_are_sorted_ranked_and_reported_without_workload_changes(
) -> Result<(), Box<dyn std::error::Error>> {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/profile-rust-tests");
    let probe = r#"
import io, json, pathlib, runpy, sys, tempfile
script = pathlib.Path(sys.argv[1])
sys.path.insert(0, str(script.parent))
module = runpy.run_path(script)
accounting = __import__("profile_rust_archive_accounting")
root = pathlib.Path(tempfile.mkdtemp())
receipts = root / "receipts"
receipts.mkdir()
records = [
    {"schema":"codexy.archive-inspector.receipt/v1","test":"suite_archive::candidate","fixture":"candidate.tar.gz","backend":"rg","started_epoch_us":30,"ended_epoch_us":55,"duration_us":25,"inspector_outcome":"success","content_comparator_ran":True,"id":"3-1"},
    {"schema":"codexy.archive-inspector.receipt/v1","test":"suite_archive::safety","fixture":"local.tar.gz","backend":"not-observed","started_epoch_us":10,"ended_epoch_us":14,"duration_us":4,"inspector_outcome":"exit:1","content_comparator_ran":False,"id":"1-1"},
    {"schema":"codexy.archive-inspector.receipt/v1","test":"suite_archive::candidate","fixture":"candidate.tar.gz","backend":"rg","started_epoch_us":20,"ended_epoch_us":45,"duration_us":25,"inspector_outcome":"success","content_comparator_ran":True,"id":"2-1"},
]
for name, record in zip(("z.json", "a.json", "m.json"), records):
    (receipts / name).write_text(json.dumps(record), encoding="utf-8")
loaded = accounting.load_archive_inspection_receipts(receipts)
ranked = accounting.rank_archive_inspection_receipts(loaded)
if [record["id"] for record in loaded] != ["2-1", "3-1", "1-1"] or ranked[0]["invocations"] != 2 or ranked[0]["total_duration_us"] != 50:
    raise SystemExit(f"loaded={loaded!r} ranked={ranked!r}")
for field in ("started_epoch_us", "ended_epoch_us", "duration_us"):
    invalid = dict(records[0])
    invalid[field] = True
    (receipts / f"invalid-{field}.json").write_text(json.dumps(invalid), encoding="utf-8")
if len(accounting.load_archive_inspection_receipts(receipts)) != len(records):
    raise SystemExit("boolean timestamp accepted")
main = module["main"].__globals__
main["enforce_workflow_contract"] = lambda *_args: None
main["archive_fixture_nested_cargo_build_count"] = lambda _root: 0
main["run_workload"] = lambda *_args: ("test result: ok. 1802 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out", 1.0, 0, {"windows-job-active-zero":"completed","cargo-root-status":"0","windows-job-pids-json":"[]","windows-job-images-json":"[]","linux-cargo-descendants-json":"not-applicable","archive-inspector-receipt-lines":accounting.receipt_report_lines(receipts),"workload-seconds":0.6,"capture-seconds":0.2,"replay-seconds":0.1})
stream, saved_stdout, saved_argv = io.StringIO(), sys.stdout, sys.argv
sys.stdout, sys.argv = stream, [str(script)]
try:
    status = module["main"]()
finally:
    sys.stdout, sys.argv = saved_stdout, saved_argv
lines = dict(line.split("\t", 1) for line in stream.getvalue().splitlines() if "\t" in line)
if status != 0 or lines.get("archive-inspector-receipts-json") != json.dumps(loaded, sort_keys=True) or lines.get("archive-inspector-rank-json") != json.dumps(ranked, sort_keys=True):
    raise SystemExit(f"status={status!r} lines={lines!r}")
"#;
    let output = Command::new("python3")
        .args(["-c", probe])
        .arg(script)
        .output()?;
    assert!(output.status.success(), "{output:?}");
    Ok(())
}

#[test]
fn real_workload_renders_receipts_before_its_tempdir_is_deleted(
) -> Result<(), Box<dyn std::error::Error>> {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/profile-rust-tests");
    let probe = r#"
import io, json, os, pathlib, runpy, sys, types
script = pathlib.Path(sys.argv[1])
sys.path.insert(0, str(script.parent))
module = runpy.run_path(script)
directories = []
processes = []
jobs = []
class Control:
    def __init__(self): self.writes, self.closed = [], False
    def write(self, value): self.writes.append(value)
    def flush(self): pass
    def close(self): self.closed = True
class Process:
    pid = 1
    def __init__(self, **kwargs):
        self.stdin, self.killed = Control(), False
        processes.append(self)
        directory = pathlib.Path(kwargs["env"]["CODEXY_TEST_ARCHIVE_INSPECT_RECEIPT_DIR"])
        directories.append(directory)
        receipt = {"schema":"codexy.archive-inspector.receipt/v1","id":"real-1","test":"suite_archive::candidate","fixture":"candidate.tar.gz","backend":"rg","started_epoch_us":10,"ended_epoch_us":30,"duration_us":20,"inspector_outcome":"success","content_comparator_ran":True}
        (directory / "real-1.json").write_text(json.dumps(receipt), encoding="utf-8")
    def poll(self): return 0
    def wait(self, timeout): return 0
    def kill(self): self.killed = True
class WindowsJob:
    def __init__(self): self.assigned, self.closed, self.terminated = None, False, False; jobs.append(self)
    def assign(self, process): self.assigned = process
    def diagnostics(self, process): return {"cargo-root-status": str(process.poll()), "windows-job-pids-json": "[]", "windows-job-images-json": "[]"}
    def wait_for_empty_until(self, _deadline): return True
    def terminate_and_wait(self): self.terminated = True
    def close(self): self.closed = True
module["subprocess"].Popen = lambda *_args, **kwargs: Process(**kwargs)
module["run_workload"].__globals__["os"] = types.SimpleNamespace(name="nt", environ=os.environ)
module["run_workload"].__globals__["WindowsJob"] = WindowsJob
_output, _elapsed, status, phases = module["run_workload"](pathlib.Path("."), 1.0)
if status != 0 or len(directories) != 1 or directories[0].exists() or len(jobs) != 1 or jobs[0].assigned is not processes[0] or not jobs[0].closed or jobs[0].terminated or processes[0].killed or processes[0].stdin.writes != [b"R"] or not processes[0].stdin.closed:
    raise SystemExit(f"status={status!r} directories={directories!r} jobs={jobs!r} processes={processes!r}")
stream, saved = io.StringIO(), sys.stdout
sys.stdout = stream
try:
    module["emit_receipt_report"](phases["archive-inspector-receipt-lines"])
finally:
    sys.stdout = saved
lines = dict(line.split("\t", 1) for line in stream.getvalue().splitlines())
if (
    "suite_archive::candidate" not in lines.get("archive-inspector-receipts-json", "")
    or "suite_archive::candidate" not in lines.get("archive-inspector-rank-json", "")
    or '"total_duration_us": 20' not in lines.get("archive-inspector-rank-json", "")
):
    raise SystemExit(f"lines={lines!r}")
"#;
    let output = Command::new("python3")
        .args(["-c", probe])
        .arg(script)
        .output()?;
    assert!(output.status.success(), "{output:?}");
    Ok(())
}

#[cfg(unix)]
#[test]
fn fixture_command_receipts_preserve_output_status_and_marker_absence(
) -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;

    let temp = tempfile::tempdir()?;
    let receipts = temp.path().join("receipts");
    let inspector = temp.path().join("inspect-release-archive");
    std::fs::create_dir(&receipts)?;
    std::fs::write(&inspector, "#!/bin/sh\nprintf stdout\nprintf stderr >&2\nexit 7\n")?;
    let mut permissions = std::fs::metadata(&inspector)?.permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&inspector, permissions)?;

    unsafe { std::env::set_var("CODEXY_TEST_ARCHIVE_INSPECT_RECEIPT_DIR", &receipts) };
    let output = crate::support::FixtureCommand::new(&inspector).output()?;
    unsafe { std::env::remove_var("CODEXY_TEST_ARCHIVE_INSPECT_RECEIPT_DIR") };

    assert_eq!(output.status.code(), Some(7));
    assert_eq!(output.stdout, b"stdout");
    assert_eq!(output.stderr, b"stderr");
    let receipt = std::fs::read_dir(receipts)?
        .filter_map(Result::ok)
        .find(|entry| entry.path().extension().is_some_and(|extension| extension == "json"))
        .ok_or("receipt missing")?;
    let receipt: serde_json::Value = serde_json::from_slice(&std::fs::read(receipt.path())?)?;
    assert_eq!(receipt["fixture"], "not-observed");
    assert_eq!(receipt["backend"], "not-observed");
    assert_eq!(receipt["content_comparator_ran"], false);
    assert_eq!(receipt["inspector_outcome"], "exit:7");
    Ok(())
}

#[cfg(windows)]
#[test]
fn windows_fixture_command_keeps_the_native_receipt_writer_path(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let receipts = temp.path().join("receipts");
    let inspector = temp.path().join("inspect-release-archive");
    std::fs::create_dir(&receipts)?;
    std::fs::write(&inspector, "#!/bin/sh\nprintf 'windows-output\\n%s\\n' \"$CODEXY_TEST_ARCHIVE_INSPECT_RECEIPT_DIR\"\nprintf windows-error >&2\nexit 7\n")?;

    unsafe { std::env::set_var("CODEXY_TEST_ARCHIVE_INSPECT_RECEIPT_DIR", &receipts) };
    let output = crate::support::FixtureCommand::new(&inspector).output()?;
    unsafe { std::env::remove_var("CODEXY_TEST_ARCHIVE_INSPECT_RECEIPT_DIR") };

    assert_eq!(output.status.code(), Some(7));
    let child_receipts = crate::support::fixture_path_text(&receipts)?;
    assert_eq!(output.stdout, format!("windows-output\n{child_receipts}\n").as_bytes());
    assert_eq!(output.stderr, b"windows-error");
    let receipt = std::fs::read_dir(receipts)?
        .filter_map(Result::ok)
        .find(|entry| entry.path().extension().is_some_and(|extension| extension == "json"))
        .ok_or("native receipt missing")?;
    let receipt: serde_json::Value = serde_json::from_slice(&std::fs::read(receipt.path())?)?;
    assert_eq!(receipt["inspector_outcome"], "exit:7");
    Ok(())
}
