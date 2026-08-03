use std::path::Path;
use std::process::Command;

#[test]
fn target_summary_recovers_one_interleaved_completed_result()
-> Result<(), Box<dyn std::error::Error>> {
    let accounting = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("scripts/profile_rust_accounting.py");
    let probe = r#"
import importlib.util, pathlib, sys

path = pathlib.Path(sys.argv[1])
spec = importlib.util.spec_from_file_location("accounting", path)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
output = "\n".join((
    "     Running tests/suites/all.rs (target/debug/deps/suite_all-a)",
    "test support::interleaved ... okSyntax error from child stderr",
    "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s",
))
tests, targets, outcomes = module.observed_test_records(output)
expected = "suite_all::support::interleaved"
if tests.get(expected) != 1 or outcomes.get("ok") != 1 or targets != {"suite_all"}:
    raise SystemExit(f"tests={tests!r} targets={targets!r} outcomes={outcomes!r}")

def assert_no_inference(label, lines):
    tests, _, outcomes = module.observed_test_records("\n".join(lines))
    if tests or outcomes:
        raise SystemExit(f"{label}: tests={tests!r} outcomes={outcomes!r}")

assert_no_inference("failed summary", (
    "     Running tests/suites/all.rs (target/debug/deps/suite_all-b)",
    "test support::failed ... child failure",
    "test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s",
    "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s",
))
assert_no_inference("ignored summary", (
    "     Running tests/suites/all.rs (target/debug/deps/suite_all-c)",
    "test support::ignored ... ignored by harness",
    "test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s",
    "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s",
))
assert_no_inference("non-adjacent summary", (
    "     Running tests/suites/all.rs (target/debug/deps/suite_all-d)",
    "test support::non_adjacent ... child output",
    "unrelated diagnostic output",
    "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s",
))
assert_no_inference("malformed summary", (
    "     Running tests/suites/all.rs (target/debug/deps/suite_all-e)",
    "test support::malformed ... child output",
    "test result: ok. passed count malformed",
    "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s",
))
assert_no_inference("target boundary", (
    "     Running tests/suites/all.rs (target/debug/deps/suite_all-f)",
    "test support::cross_target ... child output",
    "     Running tests/suites/archive.rs (target/debug/deps/suite_archive-g)",
    "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s",
))
"#;
    let output = Command::new("python3")
        .args(["-c", probe])
        .arg(accounting)
        .output()?;

    assert!(output.status.success(), "{output:?}");
    Ok(())
}
