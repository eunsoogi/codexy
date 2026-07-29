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
            "timeout-minutes: 10",
            "run: python scripts/profile-rust-tests --windows",
        ],
    );
    assert_eq!(workflow.matches("windows-rust-profile").count(), 0);
    assert_eq!(workflow.matches("scripts/profile-rust-tests").count(), 2);
}

#[test]
fn gate_accepts_only_the_exact_native_windows_workload() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    std::fs::write(
        &fixture.workflow,
        "jobs:\n  rust-test:\n    runs-on: ubuntu-latest\n    timeout-minutes: 4\n    steps:\n      - run: scripts/profile-rust-tests\n  windows-rust-test:\n    runs-on: windows-latest\n    timeout-minutes: 10\n    steps:\n      - run: scripts/install-windows-test-prerequisites.ps1\n      - run: python scripts/profile-rust-tests --windows\n",
    )?;
    assert!(fixture.run(&[])?.status.success());

    std::fs::write(
        &fixture.workflow,
        "jobs:\n  rust-test:\n    runs-on: ubuntu-latest\n    timeout-minutes: 4\n    steps:\n      - run: scripts/profile-rust-tests\n",
    )?;
    assert!(
        !fixture
            .run_without_required_windows_job(&[])?.status.success()
    );

    std::fs::write(
        &fixture.workflow,
        "jobs:\n  rust-test:\n    timeout-minutes: 4\n    steps:\n      - run: scripts/profile-rust-tests\n  windows-rust-test:\n    runs-on: windows-latest\n    timeout-minutes: 12\n    steps:\n      - run: python scripts/profile-rust-tests --windows\n",
    )?;
    assert!(!fixture.run(&[])?.status.success());

    std::fs::write(
        &fixture.workflow,
        "jobs:\n  rust-test:\n    runs-on: macos-latest\n    timeout-minutes: 4\n    steps:\n      - run: scripts/profile-rust-tests\n  windows-rust-test:\n    runs-on: windows-latest\n    timeout-minutes: 10\n    steps:\n      - run: scripts/install-windows-test-prerequisites.ps1\n      - run: python scripts/profile-rust-tests --windows\n",
    )?;
    assert!(!fixture.run(&[])?.status.success());

    std::fs::write(
        &fixture.workflow,
        "jobs:\n  rust-test:\n    runs-on: ubuntu-latest\n    strategy:\n      matrix:\n        include: [one]\n    timeout-minutes: 4\n    steps:\n      - run: scripts/profile-rust-tests\n  windows-rust-test:\n    runs-on: windows-latest\n    timeout-minutes: 10\n    steps:\n      - run: scripts/install-windows-test-prerequisites.ps1\n      - run: python scripts/profile-rust-tests --windows\n",
    )?;
    assert!(!fixture.run(&[])?.status.success());

    std::fs::write(
        &fixture.workflow,
        "jobs:\n  rust-test:\n    timeout-minutes: 4\n    steps:\n      - run: scripts/profile-rust-tests\n  windows-rust-test:\n    runs-on: windows-latest\n    timeout-minutes: 10\n    steps:\n      - run: scripts/unapproved-windows-step.ps1\n      - run: python scripts/profile-rust-tests --windows\n",
    )?;
    assert!(!fixture.run(&[])?.status.success());

    std::fs::write(
        &fixture.workflow,
        "jobs:\n  rust-test:\n    timeout-minutes: 4\n    steps:\n      - run: scripts/profile-rust-tests\n  windows-rust-test:\n    runs-on: ubuntu-latest\n    timeout-minutes: 10\n    steps:\n      - run: python scripts/profile-rust-tests --windows\n",
    )?;
    assert!(!fixture.run(&[])?.status.success());
    Ok(())
}
