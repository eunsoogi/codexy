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
        self.writes = []
    def write(self, text):
        self.writes.append(text)
    def flush(self):
        self.flushes += 1

class Stream:
    def __init__(self, chunks):
        self.chunks = list(chunks)
        self.readline_calls = 0
        self.read_calls = 0
    def readline(self):
        self.readline_calls += 1
        return self.chunks.pop(0) if self.chunks else b""
    def read(self):
        self.read_calls += 1
        remaining = b"".join(self.chunks)
        self.chunks.clear()
        return remaining

class Process:
    def __init__(self, chunks):
        self.stdout = Stream(chunks)
    def wait(self, timeout):
        return 0
    def poll(self):
        return 0

script = pathlib.Path(sys.argv[1])
sys.path.insert(0, str(script.parent))
module = runpy.run_path(script)
def measure(chunks):
    process = Process(chunks)
    module["subprocess"].Popen = lambda *args, **kwargs: process
    recorder = Recorder()
    original_stdout = sys.stdout
    sys.stdout = recorder
    try:
        output, _elapsed, status = module["run_workload"](None, 1.0)
    finally:
        sys.stdout = original_stdout
    return (
        status,
        output,
        recorder.flushes,
        process.stdout.readline_calls,
        process.stdout.read_calls,
        recorder.writes,
    )

actual = [
    measure(()),
    measure((b"first\n",)),
    measure((b"first\n", b"second\n", b"third\n")),
]
expected = [
    (0, "", 1, 1, 1, []),
    (0, "first\n", 2, 1, 1, ["first\n"]),
    (0, "first\nsecond\nthird\n", 2, 1, 1, ["first\n", "second\nthird\n"]),
]
if actual != expected:
    raise SystemExit(f"actual={actual!r} expected={expected!r}")

large_tail = (b"second\r\n" * 4096) + "lambda=λ\r\n".encode()
large = measure((b"first\r\n", large_tail))
large_expected = (
    0,
    (b"first\r\n" + large_tail).decode(),
    2,
    1,
    1,
    [b"first\r\n".decode(), large_tail.decode()],
)
if large != large_expected:
    raise SystemExit(f"large={large!r} expected={large_expected!r}")
"#;
    let output = Command::new("python3")
        .args(["-c", probe])
        .arg(script)
        .output()?;

    assert!(output.status.success(), "{output:?}");
    Ok(())
}
