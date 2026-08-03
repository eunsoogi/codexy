use std::path::Path;

use super::super::GateFixture;

const RUST_JOB: &str =
    "    runs-on: ubuntu-latest\n    timeout-minutes: 6\n    steps:\n      - run: scripts/profile-rust-tests\n";
const WINDOWS_STEPS: &str =
    "      - run: scripts/install-windows-test-prerequisites.ps1\n      - run: |\n          rustup toolchain install\n          if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }\n          cargo fetch --locked\n          if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }\n      - run: python scripts/profile-rust-tests --windows\n";
const NESTED_BLOCK_STEPS: &str = "      - name: Archive prerequisite\n        run: scripts/install-windows-test-prerequisites.ps1\n      - name: Bootstrap\n\n        # The parser keeps step metadata separate from the run scalar.\n        run: |\n          rustup toolchain install\n          if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }\n          cargo fetch --locked\n          if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }\n\n      - name: Profile\n        run: >\n          python scripts/profile-rust-tests --windows\n";

fn workflow(rust_job: &str, windows_runner: &str, timeout: u8, steps: &str) -> String {
    format!(
        "jobs:\n  rust-test:\n{rust_job}  windows-rust-test:\n    runs-on: {windows_runner}\n    timeout-minutes: {timeout}\n    steps:\n{steps}"
    )
}

#[path = "windows_workflow_contract_matrix.rs"]
mod matrix;

#[test]
fn rust_workflow_runs_the_full_suite_natively_on_windows() {
    let workflow = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/rust-test.yml"),
    )
    .expect("read Rust workflow");

    crate::support::assert_structured_literals(
        &workflow,
        "rust-workflow-windows-suite",
        &[
            "windows-rust-test:",
            "name: Rust test suite (Windows)",
            "runs-on: windows-latest",
            "timeout-minutes: 20",
            "name: Prepare Windows Rust toolchain",
            "run: |\n          rustup toolchain install\n          if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }\n          cargo fetch --locked\n          if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }",
            "run: python scripts/profile-rust-tests --windows",
        ],
    );
    assert_eq!(workflow.matches("windows-rust-profile").count(), 0);
    assert_eq!(workflow.matches("scripts/profile-rust-tests").count(), 2);
}

#[test]
fn gate_retains_the_exact_native_windows_positive() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    std::fs::write(
        &fixture.workflow,
        workflow(RUST_JOB, "windows-latest", 20, WINDOWS_STEPS),
    )?;
    assert!(fixture.run(&[])?.status.success());
    Ok(())
}

#[test]
fn gate_retains_the_missing_windows_job_negative() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    std::fs::write(&fixture.workflow, format!("jobs:\n  rust-test:\n{RUST_JOB}"))?;
    assert!(!fixture.run_without_required_windows_job(&[])?.status.success());
    Ok(())
}
