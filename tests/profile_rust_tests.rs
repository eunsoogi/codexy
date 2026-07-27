#[cfg(unix)]
#[path = "profile_rust_tests/workflow_contract.rs"]
mod workflow_contract;

#[cfg(unix)]
#[path = "profile_rust_tests/gate_fixture.rs"]
mod gate_fixture;

#[cfg(unix)]
pub(super) use gate_fixture::GateFixture;

#[cfg(unix)]
#[path = "profile_rust_tests/gate_output.rs"]
mod gate_output;

#[cfg(unix)]
#[path = "profile_rust_tests/windows_accounting.rs"]
mod windows_accounting;

#[cfg(unix)]
#[test]
fn gate_propagates_a_single_full_workload_failure() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(42, 1802, 0)?;
    let output = fixture.run(&[])?;

    assert_eq!(output.status.code(), Some(42), "{output:?}");
    assert_eq!(
        std::fs::read_to_string(&fixture.marker)?,
        "test --locked --all-targets\n"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn gate_fails_an_exact_workload_over_the_budget_without_sleeping()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    let clock = fixture.temp.path().join("clock");
    std::fs::create_dir(&clock)?;
    std::fs::write(
        clock.join("sitecustomize.py"),
        "import time\n_values = iter((0.0, 0.0, 0.0, 196.0, 196.0))\ntime.perf_counter = lambda: next(_values)\n",
    )?;
    let output = fixture.run(&[("PYTHONPATH", clock.as_os_str())])?;

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert_eq!(
        std::fs::read_to_string(&fixture.marker)?,
        "test --locked --all-targets\n"
    );
    assert!(String::from_utf8(output.stdout)?.contains("result\tFAIL"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn gate_rejects_coverage_loss_and_ignored_tests() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1801, 1)?;
    let output = fixture.run(&[])?;

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains("tests\t1801 passed\t0 failed\t1 ignored\tFAIL"),
        "{stdout}"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn gate_has_no_relaxable_budget_option() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    let output = fixture.run(&[("EXTRA_ARGUMENT", std::ffi::OsStr::new("--max-seconds"))])?;

    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(String::from_utf8(output.stderr)?.contains("unrecognized arguments"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn gate_rejects_timeout_or_profiler_in_an_unrelated_job() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    std::fs::write(
        &fixture.workflow,
        "jobs:\n  unrelated:\n    timeout-minutes: 4\n    steps:\n      - run: scripts/profile-rust-tests\n  rust-test:\n    timeout-minutes: 10\n    steps:\n      - run: echo not-the-gate\n",
    )?;
    assert!(!fixture.run(&[])?.status.success());
    Ok(())
}

#[cfg(unix)]
#[test]
fn gate_rejects_a_second_profiler_invocation_in_another_job(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    std::fs::write(
        &fixture.workflow,
        "jobs:\n  rust-test:\n    timeout-minutes: 4\n    steps:\n      - run: scripts/profile-rust-tests\n  unrelated:\n    steps:\n      - run: scripts/profile-rust-tests\n",
    )?;
    assert!(!fixture.run(&[])?.status.success());
    Ok(())
}

#[cfg(unix)]
#[test]
fn gate_rejects_contract_fields_leaked_from_an_underscore_job(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    std::fs::write(
        &fixture.workflow,
        "jobs:\n  rust-test:\n    steps:\n      - run: echo not-the-gate\n  _unrelated:\n    timeout-minutes: 4\n    steps:\n      - run: scripts/profile-rust-tests\n",
    )?;
    assert!(!fixture.run(&[])?.status.success());
    Ok(())
}

#[cfg(unix)]
#[test]
fn gate_rejects_a_block_scalar_that_only_mentions_the_profiler(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    std::fs::write(
        &fixture.workflow,
        "jobs:\n  rust-test:\n    timeout-minutes: 4\n    env: |\n      run: scripts/profile-rust-tests\n    steps:\n      - run: echo not-the-gate\n",
    )?;
    assert!(!fixture.run(&[])?.status.success());
    Ok(())
}

#[cfg(unix)]
#[test]
fn gate_accepts_the_profiler_step_after_a_blank_line() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    std::fs::write(
        &fixture.workflow,
        "jobs:\n  rust-test:\n    timeout-minutes: 4\n    steps:\n      - name: setup\n        run: echo setup\n\n      - run: scripts/profile-rust-tests\n",
    )?;
    assert!(fixture.run(&[])?.status.success());
    std::fs::write(
        &fixture.workflow,
        "jobs:\n  rust-test:\n    timeout-minutes: 4\n    steps:\n      - run: |\n          scripts/profile-rust-tests\n",
    )?;
    assert!(fixture.run(&[])?.status.success());
    Ok(())
}

#[cfg(unix)]
#[test]
fn gate_handles_profiler_block_scalars_and_jobs_comments() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    std::fs::write(
        &fixture.workflow,
        "jobs:\n  rust-test:\n    timeout-minutes: 4\n    steps:\n      - run: scripts/profile-rust-tests\n  unrelated:\n    steps:\n      - run: |\n          scripts/profile-rust-tests\n",
    )?;
    assert!(!fixture.run(&[])?.status.success());
    std::fs::write(
        &fixture.workflow,
        "jobs:\n# keep this ordinary comment inside the jobs mapping\n  rust-test:\n    timeout-minutes: 4\n    steps:\n      - run: scripts/profile-rust-tests\n",
    )?;
    assert!(fixture.run(&[])?.status.success());
    Ok(())
}
