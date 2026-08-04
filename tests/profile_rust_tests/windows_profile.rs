use std::path::Path;
use std::process::Command;

#[test]
fn windows_profiler_bounds_default_parallelism_without_overriding_callers()
-> Result<(), Box<dyn std::error::Error>> {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/profile-rust-tests");
    let probe = r##"
import json, os, pathlib, runpy, sys, tempfile

script = pathlib.Path(sys.argv[1])
workspace = pathlib.Path(tempfile.mkdtemp())
root = workspace / "root"
root.mkdir()
bin_dir = workspace / "bin"
bin_dir.mkdir()
marker = workspace / "cargo-marker"
cargo = bin_dir / "cargo"
cargo.write_text("#!/bin/sh\nprintf '%s|%s|%s|%s\\n' \"${RUST_TEST_THREADS-}\" \"${CARGO_PROFILE_TEST_DEBUG-}\" \"${CARGO_PROFILE_TEST_OPT_LEVEL-}\" \"$*\" >> \"$PROFILE_MARKER\"\nprintf '%s\\n' 'test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out'\n")
cargo.chmod(0o755)
sys.path.insert(0, str(script.parent))
module = runpy.run_path(script)
run_workload = module["run_workload"]

base = dict(os.environ)
base["PATH"] = str(bin_dir) + os.pathsep + base["PATH"]
base["PROFILE_MARKER"] = str(marker)
base["RUNNER_TEMP"] = str(workspace)
for key in ("RUST_TEST_THREADS", "CARGO_PROFILE_TEST_DEBUG", "CARGO_PROFILE_TEST_OPT_LEVEL"):
    base.pop(key, None)
os.environ.clear(); os.environ.update(base)
_output, _elapsed, status, phases = run_workload(root, 5.0, windows=True)
if status != 0:
    raise SystemExit(f"windows status={status!r}")
receipt = json.loads(phases["workload-receipt-json"])
if receipt["test_threads"] != {"state":"configured", "value":"2"}:
    raise SystemExit(f"windows thread receipt={receipt!r}")
if "cargo_test_profile" in receipt:
    raise SystemExit(f"windows profile receipt={receipt!r}")
if marker.read_text().splitlines() != ["2|||test --locked --all-targets"]:
    raise SystemExit(f"windows cargo={marker.read_text()!r}")

marker.unlink()
os.environ.clear(); os.environ.update(base)
_output, _elapsed, status, phases = run_workload(root, 5.0, windows=False)
if status != 0:
    raise SystemExit(f"non-windows status={status!r}")
receipt = json.loads(phases["workload-receipt-json"])
if receipt["test_threads"] != {"state":"default/unobserved", "value":"not-observed"}:
    raise SystemExit(f"non-windows thread receipt={receipt!r}")
if "cargo_test_profile" in receipt:
    raise SystemExit(f"non-windows profile receipt={receipt!r}")
if marker.read_text().splitlines() != ["|||test --locked --all-targets"]:
    raise SystemExit(f"non-windows cargo={marker.read_text()!r}")

marker.unlink()
configured = dict(base)
configured["RUST_TEST_THREADS"] = "3"
os.environ.clear(); os.environ.update(configured)
_output, _elapsed, status, phases = run_workload(root, 5.0, windows=True)
if status != 0:
    raise SystemExit(f"configured windows status={status!r}")
receipt = json.loads(phases["workload-receipt-json"])
if receipt["test_threads"] != {"state":"configured", "value":"3"}:
    raise SystemExit(f"configured windows thread receipt={receipt!r}")
if marker.read_text().splitlines() != ["3|||test --locked --all-targets"]:
    raise SystemExit(f"configured windows cargo={marker.read_text()!r}")

"##;
    let output = Command::new("python3")
        .args(["-c", probe])
        .arg(script)
        .output()?;
    assert!(output.status.success(), "{output:?}");
    Ok(())
}
