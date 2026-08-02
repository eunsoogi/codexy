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
(metrics / "interval-p12-1.metrics").write_text(row("p12-1", 1, 0, 10_000_000_000))
payload = json.loads(module.telemetry(None, {}, None, interval_metrics_path=metrics))
if payload["command_interval_ranked"][0]["conservative_union_occupancy_seconds"] != 12.0:
    raise SystemExit("cross-process clocks were merged")
one.write_text(row("p11-1", 1, 0, 10_000_000_000) + row("p11-1", 2, 20_000_000_000, 30_000_000_000))
payload = json.loads(module.telemetry(None, {}, None, interval_metrics_path=metrics))
if payload["command_interval_ranked"][0]["conservative_union_occupancy_seconds"] != 20.0:
    raise SystemExit("disjoint local union was not exact")
if "/" in json.dumps(payload) or "session" in json.dumps(payload["command_interval_ranked"]):
    raise SystemExit("unsafe interval data leaked")
for invalid in [
    row("p11-1", 1, 11, 10),
    row("p11-1", 1, 0, 181_000_000_000),
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
