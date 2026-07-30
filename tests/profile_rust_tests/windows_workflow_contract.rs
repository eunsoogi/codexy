use std::path::Path;

use super::super::GateFixture;

const RUST_JOB: &str =
    "    runs-on: ubuntu-latest\n    timeout-minutes: 4\n    steps:\n      - run: scripts/profile-rust-tests\n";
const WINDOWS_STEPS: &str =
    "      - run: scripts/install-windows-test-prerequisites.ps1\n      - run: |\n          rustup toolchain install\n          cargo fetch --locked\n      - run: python scripts/profile-rust-tests --windows\n";
const NESTED_BLOCK_STEPS: &str = "      - name: Archive prerequisite\n        run: scripts/install-windows-test-prerequisites.ps1\n      - name: Bootstrap\n\n        # The parser keeps step metadata separate from the run scalar.\n        run: |\n          rustup toolchain install\n          cargo fetch --locked\n\n      - name: Profile\n        run: >\n          python scripts/profile-rust-tests --windows\n";

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
            "run: |\n          rustup toolchain install\n          cargo fetch --locked",
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
    let steps = "      - run: scripts/install-windows-test-prerequisites.ps1\n      - run: |\n          rustup toolchain install\n      - run: python scripts/profile-rust-tests --windows\n";
    std::fs::write(
        &fixture.workflow,
        workflow(RUST_JOB, "windows-latest", 20, steps),
    )?;

    assert!(!fixture.run(&[])?.status.success());
    Ok(())
}

#[test]
fn gate_rejects_missing_misordered_or_unlocked_windows_prefetch(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    for steps in [
        "      - run: scripts/install-windows-test-prerequisites.ps1\n      - run: |\n          rustup toolchain install\n      - run: python scripts/profile-rust-tests --windows\n",
        "      - run: scripts/install-windows-test-prerequisites.ps1\n      - run: |\n          rustup toolchain install\n          cargo fetch\n      - run: python scripts/profile-rust-tests --windows\n",
        "      - run: scripts/install-windows-test-prerequisites.ps1\n      - run: |\n          rustup toolchain install\n          cargo fetch --locked\n      - run: python scripts/profile-rust-tests --windows\n      - run: cargo fetch --locked\n",
        "      - run: scripts/install-windows-test-prerequisites.ps1\n      - run: |\n          cargo test --locked --all-targets\n          rustup toolchain install\n          cargo fetch --locked\n      - run: python scripts/profile-rust-tests --windows\n",
    ] {
        std::fs::write(&fixture.workflow, workflow(RUST_JOB, "windows-latest", 20, steps))?;
        assert!(!fixture.run(&[])?.status.success());
    }
    Ok(())
}

#[test]
fn gate_accepts_required_windows_runs_in_nested_block_scalars(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    std::fs::write(
        &fixture.workflow,
        workflow(RUST_JOB, "windows-latest", 20, NESTED_BLOCK_STEPS),
    )?;

    assert!(fixture.run(&[])?.status.success());
    Ok(())
}

#[test]
fn gate_rejects_empty_or_misaligned_folded_runs_without_parser_errors(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    for bootstrap in ["        run: >\n", "        run: >\n        rustup toolchain install\n"] {
        let steps = format!(
            "      - run: scripts/install-windows-test-prerequisites.ps1\n      - name: Bootstrap\n{bootstrap}      - run: python scripts/profile-rust-tests --windows\n"
        );
        std::fs::write(
            &fixture.workflow,
            workflow(RUST_JOB, "windows-latest", 20, &steps),
        )?;
        let output = fixture.run(&[])?;
        assert!(!output.status.success());
        assert!(!String::from_utf8_lossy(&output.stderr).contains("ValueError"));
    }
    Ok(())
}

#[test]
fn gate_rejects_skipped_or_fail_open_required_windows_steps(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    for control in ["if: false", "continue-on-error: true", "\"if\": false", "'continue-on-error': true"] {
        let steps = WINDOWS_STEPS.replacen(
            "      - run: python scripts/profile-rust-tests --windows",
            &format!("        {control}\n      - run: python scripts/profile-rust-tests --windows"),
            1,
        );
        std::fs::write(
            &fixture.workflow,
            workflow(RUST_JOB, "windows-latest", 20, &steps),
        )?;
        assert!(!fixture.run(&[])?.status.success());
    }
    for control in ["if: false", "continue-on-error: true"] {
        let workflow = workflow(RUST_JOB, "windows-latest", 20, WINDOWS_STEPS).replacen(
            "    timeout-minutes: 20",
            &format!("    {control}\n    timeout-minutes: 20"),
            1,
        );
        std::fs::write(&fixture.workflow, workflow)?;
        assert!(!fixture.run(&[])?.status.success());
    }
    Ok(())
}
