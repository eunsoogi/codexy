use std::path::Path;

use super::super::GateFixture;

const RUST_JOB: &str =
    "    runs-on: ubuntu-latest\n    timeout-minutes: 4\n    steps:\n      - run: scripts/profile-rust-tests\n";
const WINDOWS_STEPS: &str =
    "      - run: scripts/install-windows-test-prerequisites.ps1\n      - run: rustup toolchain install\n      - run: python scripts/profile-rust-tests --windows\n";

fn workflow(rust_job: &str, windows_runner: &str, timeout: u8, steps: &str) -> String {
    format!(
        "jobs:\n  rust-test:\n{rust_job}  windows-rust-test:\n    runs-on: {windows_runner}\n    timeout-minutes: {timeout}\n    steps:\n{steps}"
    )
}

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
            "run: rustup toolchain install",
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
        workflow(RUST_JOB, "windows-latest", 20, WINDOWS_STEPS),
    )?;
    assert!(fixture.run(&[])?.status.success());

    for timeout in [10, 19, 21] {
        std::fs::write(
            &fixture.workflow,
            workflow(RUST_JOB, "windows-latest", timeout, WINDOWS_STEPS),
        )?;
        assert!(!fixture.run(&[])?.status.success());
    }

    std::fs::write(&fixture.workflow, format!("jobs:\n  rust-test:\n{RUST_JOB}"))?;
    assert!(
        !fixture
            .run_without_required_windows_job(&[])?.status.success()
    );

    let rust_matrix = "    runs-on: ubuntu-latest\n    strategy:\n      matrix:\n        include: [one]\n    timeout-minutes: 4\n    steps:\n      - run: scripts/profile-rust-tests\n";
    for (rust_job, runner, steps) in [
        (
            "    runs-on: macos-latest\n    timeout-minutes: 4\n    steps:\n      - run: scripts/profile-rust-tests\n",
            "windows-latest",
            WINDOWS_STEPS,
        ),
        (rust_matrix, "windows-latest", WINDOWS_STEPS),
        (
            RUST_JOB,
            "windows-latest",
            "      - run: scripts/unapproved-windows-step.ps1\n      - run: python scripts/profile-rust-tests --windows\n",
        ),
        (RUST_JOB, "ubuntu-latest", WINDOWS_STEPS),
    ] {
        std::fs::write(&fixture.workflow, workflow(rust_job, runner, 20, steps))?;
        assert!(!fixture.run(&[])?.status.success());
    }
    Ok(())
}

#[test]
fn gate_rejects_a_windows_profile_without_prior_toolchain_bootstrap(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    let steps = "      - run: scripts/install-windows-test-prerequisites.ps1\n      - run: python scripts/profile-rust-tests --windows\n";
    std::fs::write(
        &fixture.workflow,
        workflow(RUST_JOB, "windows-latest", 20, steps),
    )?;

    assert!(!fixture.run(&[])?.status.success());
    Ok(())
}

#[test]
fn gate_rejects_skipped_or_fail_open_required_windows_steps(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    for steps in [
        "      - run: scripts/install-windows-test-prerequisites.ps1\n      - run: rustup toolchain install\n        if: false\n      - run: python scripts/profile-rust-tests --windows\n",
        "      - run: scripts/install-windows-test-prerequisites.ps1\n      - run: rustup toolchain install\n        continue-on-error: true\n      - run: python scripts/profile-rust-tests --windows\n",
        "      - run: scripts/install-windows-test-prerequisites.ps1\n      - run: rustup toolchain install\n      - run: python scripts/profile-rust-tests --windows\n        if: false\n",
        "      - run: scripts/install-windows-test-prerequisites.ps1\n      - run: rustup toolchain install\n      - run: python scripts/profile-rust-tests --windows\n        continue-on-error: true\n",
    ] {
        std::fs::write(
            &fixture.workflow,
            workflow(RUST_JOB, "windows-latest", 20, steps),
        )?;
        assert!(!fixture.run(&[])?.status.success());
    }
    Ok(())
}
