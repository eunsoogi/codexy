use std::path::Path;
use std::process::Command;

#[test]
fn profiler_reports_target_scoped_interval_union_without_paths()
-> Result<(), Box<dyn std::error::Error>> {
    let helper = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/profile_rust_telemetry.py");
    let probe = r#"
import importlib.util, json, pathlib, sys, tempfile

path = pathlib.Path(sys.argv[1])
sys.path.insert(0, str(path.parent))
spec = importlib.util.spec_from_file_location("telemetry", path)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
metrics = pathlib.Path(tempfile.mkdtemp())
session = "0123456789abcdef0123456789abcdef"
def row(producer, sequence, start, end, key="fixture-command.output", family="python"):
    return f"command-interval\tv2\t{session}\tsuite_all\t{producer}\t{sequence}\t{key}\t{family}\t{start}\t{end}\n"
def owner_row(producer, sequence, source, line, start, end, family="python"):
    return f"fixture-command-owner\tv1\t{session}\tsuite_all\t{producer}\t{sequence}\tfixture-command.output\t{family}\t{source}\t{line}\t{start}\t{end}\n"
one = metrics / "interval-p11-1.metrics"
one.write_text(row("p11-1", 1, 0, 10_000_000_000) + row("p11-1", 2, 2_000_000_000, 12_000_000_000))
payload = json.loads(module.telemetry(None, {}, None, interval_metrics_path=metrics))
expected = {
    "target":"suite_all", "key":"fixture-command.output", "family":"python",
    "count":2, "producer_count":1, "cumulative_wait_seconds":20.0,
    "conservative_union_occupancy_seconds":12.0, "overlap_ratio_upper_bound":0.4,
}
if payload["command_interval_ranked"] != [expected]:
    raise SystemExit(payload)
if payload["command_interval_owner_ranked"] or payload["command_interval_owner_coverage"]["records"] != 0:
    raise SystemExit("absent owner data changed legacy profiling")
owners = metrics.parent / (metrics.name + "-owners")
owners.mkdir()
(owners / "owner-interval-p11-1.metrics").write_text(
    owner_row("p11-1", 1, "tests/fixture.rs", 7, 0, 10_000_000_000)
    + owner_row("p11-1", 2, "tests/fixture.rs", 7, 2_000_000_000, 12_000_000_000)
)
payload = json.loads(module.telemetry(None, {}, None, interval_metrics_path=metrics, interval_owner_metrics_path=owners))
owner_expected = {
    "target":"suite_all", "key":"fixture-command.output", "family":"python",
    "owner":"tests/fixture.rs:7", "count":2, "producer_count":1,
    "cumulative_wait_seconds":20.0, "conservative_union_occupancy_seconds":12.0,
    "overlap_ratio_upper_bound":0.4,
}
if payload["command_interval_owner_ranked"] != [owner_expected]:
    raise SystemExit(payload["command_interval_owner_ranked"])
if payload["command_interval_owner_coverage"] != {
    "records":2, "expected_records":2, "groups":1, "unattributed":0, "truncated":False
}:
    raise SystemExit(payload["command_interval_owner_coverage"])
if "runner" in json.dumps(payload["command_interval_owner_ranked"]):
    raise SystemExit("absolute owner path leaked")
(metrics / "interval-p12-1.metrics").write_text(row("p12-1", 1, 0, 10_000_000_000))
(owners / "owner-interval-p12-1.metrics").write_text(
    owner_row("p12-1", 1, "tests/fixture.rs", 7, 0, 10_000_000_000)
)
payload = json.loads(module.telemetry(None, {}, None, interval_metrics_path=metrics, interval_owner_metrics_path=owners))
if payload["command_interval_owner_ranked"][0]["producer_count"] != 2 or payload["command_interval_owner_ranked"][0]["conservative_union_occupancy_seconds"] != 12.0:
    raise SystemExit("owner intervals merged across producer clocks")
payload = json.loads(module.telemetry(None, {}, None, interval_metrics_path=metrics))
if payload["command_interval_ranked"][0]["conservative_union_occupancy_seconds"] != 12.0:
    raise SystemExit("cross-process clocks were merged")
one.write_text(row("p11-1", 1, 0, 10_000_000_000) + row("p11-1", 2, 20_000_000_000, 30_000_000_000))
payload = json.loads(module.telemetry(None, {}, None, interval_metrics_path=metrics))
if payload["command_interval_ranked"][0]["conservative_union_occupancy_seconds"] != 20.0:
    raise SystemExit("disjoint local union was not exact")
one.write_text(row("p11-1", 1, 0, 240_000_000_000))
payload = json.loads(module.telemetry(None, {}, None, interval_metrics_path=metrics))
if payload["command_interval_ranked"][0]["conservative_union_occupancy_seconds"] != 240.0:
    raise SystemExit("valid interval above 180 seconds was rejected")
one.write_text(row("p11-1", 1, 0, 300_000_000_000))
payload = json.loads(module.telemetry(None, {}, None, interval_metrics_path=metrics))
if payload["command_interval_ranked"][0]["conservative_union_occupancy_seconds"] != 300.0:
    raise SystemExit("exact 300-second interval was rejected")
if "/" in json.dumps(payload) or "session" in json.dumps(payload["command_interval_ranked"]):
    raise SystemExit("unsafe interval data leaked")
one.write_text(row("p11-1", 1, 0, 300_000_000_001))
try:
    module.telemetry(None, {}, None, interval_metrics_path=metrics)
except ValueError as error:
    if str(error) != "invalid interval bounds":
        raise
else:
    raise SystemExit("accepted interval above 300 seconds")
for invalid in [
    row("p11-1", 1, 11, 10),
    row("p11-1", 1, -1, 10),
    row("p11-1", "not-an-integer", 0, 10),
    row("p11-1", 1, 0, 10, "unknown"),
    row("p11-1", 1, 0, 10, "wrapper.output.git", "python"),
    row("p11-1", 1, 0, 10) + row("p11-1", 1, 10, 20),
    row("p11-1", 1, 0, 10).rstrip("\n") + "\textra\n",
]:
    one.write_text(invalid)
    try:
        module.telemetry(None, {}, None, interval_metrics_path=metrics)
    except ValueError:
        continue
    raise SystemExit(f"accepted invalid interval record: {invalid!r}")
one.write_text(row("p11-1", 1, 0, 10) + row("p11-1", 2, 10, 20))
(metrics / "interval-p12-1.metrics").write_text(row("p12-1", 1, 0, 10))
for invalid_owner in [
    owner_row("p11-1", 1, "/runner/tests/fixture.rs", 7, 0, 10),
    owner_row("p11-1", 1, "tests/../fixture.rs", 7, 0, 10),
    owner_row("p11-1", 1, "tests/fixture.rs", 0, 0, 10),
]:
    (owners / "owner-interval-p11-1.metrics").write_text(invalid_owner)
    try:
        module.telemetry(None, {}, None, interval_metrics_path=metrics, interval_owner_metrics_path=owners)
    except ValueError:
        continue
    raise SystemExit(f"accepted invalid owner interval record: {invalid_owner!r}")
if json.loads(module.telemetry(None, {}, None))["command_interval_ranked"]:
    raise SystemExit("disabled transport fabricated records")
"#;
    let output = Command::new("python3")
        .args(["-c", probe])
        .arg(helper)
        .output()?;
    assert!(output.status.success(), "{output:?}");
    Ok(())
}
