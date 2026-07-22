use std::path::Path;

use super::super::GateFixture;

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
            "run: cargo test --locked --all-targets",
        ],
    );
    assert_eq!(workflow.matches("cargo test --locked --all-targets").count(), 1);
    assert_eq!(workflow.matches("scripts/profile-rust-tests").count(), 1);
}

#[test]
fn gate_accepts_only_the_exact_native_windows_workload() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    std::fs::write(
        &fixture.workflow,
        "jobs:\n  rust-test:\n    timeout-minutes: 4\n    steps:\n      - run: scripts/profile-rust-tests\n  windows-rust-test:\n    runs-on: windows-latest\n    steps:\n      - run: cargo test --locked --all-targets\n",
    )?;
    assert!(fixture.run(&[])?.status.success());

    std::fs::write(
        &fixture.workflow,
        "jobs:\n  rust-test:\n    timeout-minutes: 4\n    steps:\n      - run: scripts/profile-rust-tests\n  windows-rust-test:\n    runs-on: ubuntu-latest\n    steps:\n      - run: cargo test --locked --all-targets\n",
    )?;
    assert!(!fixture.run(&[])?.status.success());
    Ok(())
}
