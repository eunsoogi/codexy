#[cfg(unix)]
use super::GateFixture;

#[cfg(unix)]
#[test]
fn gate_rejects_two_profiler_commands_in_one_block_run() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    std::fs::write(
        &fixture.workflow,
        "jobs:\n  rust-test:\n    timeout-minutes: 4\n    steps:\n      - run: |\n          scripts/profile-rust-tests\n          scripts/profile-rust-tests\n",
    )?;

    assert!(!fixture.run(&[])?.status.success());
    Ok(())
}

#[cfg(unix)]
#[test]
fn gate_rejects_whitespace_variant_full_workload_outside_its_gate()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    std::fs::write(
        &fixture.workflow,
        "jobs:\n  rust-test:\n    timeout-minutes: 4\n    steps:\n      - run: scripts/profile-rust-tests\n      - run: cargo  test --locked --all-targets\n",
    )?;

    assert!(!fixture.run(&[])?.status.success());
    Ok(())
}

#[cfg(unix)]
#[test]
fn gate_ignores_a_comment_that_mentions_the_full_workload()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    std::fs::write(
        &fixture.workflow,
        "jobs:\n  rust-test:\n    timeout-minutes: 4\n    steps:\n      # cargo test --locked --all-targets stays inside the profiler\n      - run: scripts/profile-rust-tests\n",
    )?;

    assert!(fixture.run(&[])?.status.success());
    Ok(())
}
