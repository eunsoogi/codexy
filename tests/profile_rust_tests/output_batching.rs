use std::path::Path;
use std::process::Command;

#[test]
fn gate_flushes_the_first_live_line_and_batches_the_remaining_output(
) -> Result<(), Box<dyn std::error::Error>> {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/profile-rust-tests");
    let probe = r#"
import pathlib
import runpy
import sys

class Recorder:
    def __init__(self):
        self.flushes = 0
    def write(self, _text):
        pass
    def flush(self):
        self.flushes += 1

class Process:
    def __init__(self, lines):
        self.stdout = iter(lines)
    def wait(self, timeout):
        return 0
    def poll(self):
        return 0

script = pathlib.Path(sys.argv[1])
sys.path.insert(0, str(script.parent))
module = runpy.run_path(script)
def measure(lines):
    module["subprocess"].Popen = lambda *args, **kwargs: Process(lines)
    recorder = Recorder()
    original_stdout = sys.stdout
    sys.stdout = recorder
    try:
        output, _elapsed, status = module["run_workload"](None, 1.0)
    finally:
        sys.stdout = original_stdout
    return status, output, recorder.flushes

actual = [
    measure(()),
    measure(("first\n",)),
    measure(("first\n", "second\n", "third\n")),
]
expected = [
    (0, "", 1),
    (0, "first\n", 2),
    (0, "first\nsecond\nthird\n", 2),
]
if actual != expected:
    raise SystemExit(f"actual={actual!r} expected={expected!r}")
"#;
    let output = Command::new("python3")
        .args(["-c", probe])
        .arg(script)
        .output()?;

    assert!(output.status.success(), "{output:?}");
    Ok(())
}
