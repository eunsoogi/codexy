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
"#;
    let output = Command::new("python3")
        .args(["-c", probe])
        .arg(accounting)
        .output()?;

    assert!(output.status.success(), "{output:?}");
    Ok(())
}
