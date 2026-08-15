use std::process::Command;

#[test]
fn gate_flushes_the_first_live_line_and_batches_the_remaining_output(
) -> Result<(), Box<dyn std::error::Error>> {
    let script = codexy_runtime::paths::repository_root().join("scripts/profile_rust_tests.py");
    let probe = r#"
import pathlib
import runpy
import sys
import time

class Recorder:
    def __init__(self):
        self.flushes = 0
        self.writes = []
    def write(self, text):
        self.writes.append(text)
    def flush(self):
        self.flushes += 1

class Process:
    def __init__(self, chunks, output):
        self.finished = False
        output.write(b"".join(chunks))
        output.flush()
    def wait(self, timeout):
        time.sleep(0.05)
        self.finished = True
        return 0
    def poll(self):
        return 0 if self.finished else None

class WindowsTextSink:
    def __init__(self):
        self.bytes = bytearray()
    def write(self, text):
        self.bytes.extend(text.replace("\n", "\r\n").encode())
    def flush(self):
        pass

class BinarySink:
    def __init__(self):
        self.bytes = bytearray()
        self.flushes = 0
    def write(self, data):
        self.bytes.extend(data)
    def flush(self):
        self.flushes += 1

class BinaryStdout:
    def __init__(self):
        self.buffer = BinarySink()

script = pathlib.Path(sys.argv[1])
sys.path.insert(0, str(script.parent))
module = runpy.run_path(script)
payload = b"first\r\nsecond\r\n"
legacy = WindowsTextSink()
legacy.write(payload.decode())
if bytes(legacy.bytes) != b"first\r\r\nsecond\r\r\n":
    raise SystemExit(f"legacy Windows translation was not reproduced: {legacy.bytes!r}")
original_stdout = sys.stdout
try:
    fallback = WindowsTextSink()
    sys.stdout = fallback
    if module["replay_output"](payload) is not False:
        raise SystemExit("text-only fallback claimed byte-preserving replay")
    binary = BinaryStdout()
    sys.stdout = binary
    if module["replay_output"](payload) is not True:
        raise SystemExit("binary sink did not declare byte-preserving replay")
    module["flush_output"]()
finally:
    sys.stdout = original_stdout
if bytes(binary.buffer.bytes) != payload or binary.buffer.flushes != 1:
    raise SystemExit(f"binary replay was not exact: {binary.buffer.bytes!r} flushes={binary.buffer.flushes}")
def measure(chunks):
    def spawn(*_args, **kwargs):
        if kwargs["stdout"] is module["subprocess"].PIPE:
            raise SystemExit("workload capture must not use subprocess.PIPE")
        if kwargs["stderr"] is not module["subprocess"].STDOUT:
            raise SystemExit("workload capture must combine stderr into the capture file")
        return Process(chunks, kwargs["stdout"])
    module["subprocess"].Popen = spawn
    recorder = Recorder()
    original_stdout = sys.stdout
    sys.stdout = recorder
    try:
        output, _elapsed, status, _phases = module["run_workload"](None, 1.0)
    finally:
        sys.stdout = original_stdout
    return (
        status,
        output,
        recorder.flushes,
        recorder.writes,
    )

actual = [
    measure(()),
    measure((b"first\n",)),
    measure((b"first\n", b"second\n", b"third\n")),
]
expected = [
    (0, "", 1, []),
    (0, "first\n", 2, ["first\n"]),
    (0, "first\nsecond\nthird\n", 2, ["first\n", "second\nthird\n"]),
]
if actual != expected:
    raise SystemExit(f"actual={actual!r} expected={expected!r}")

large_tail = (b"second\r\n" * 4096) + "lambda=λ\r\n".encode()
large = measure((b"first\r\n", large_tail))
large_expected = (
    0,
    (b"first\r\n" + large_tail).decode(),
    2,
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
