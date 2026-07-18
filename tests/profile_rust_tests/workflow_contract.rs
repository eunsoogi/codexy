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

#[cfg(unix)]
#[test]
fn gate_does_not_count_a_folded_profiler_token_as_a_command()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    std::fs::write(
        &fixture.workflow,
        "jobs:\n  rust-test:\n    timeout-minutes: 4\n    steps:\n      - run: >\n          echo setup\n          scripts/profile-rust-tests\n",
    )?;

    assert!(!fixture.run(&[])?.status.success());
    Ok(())
}

#[cfg(unix)]
#[test]
fn gate_rejects_a_full_workload_after_a_folded_scalar_paragraph_break(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    std::fs::write(
        &fixture.workflow,
        "jobs:\n  rust-test:\n    timeout-minutes: 4\n    steps:\n      - run: >\n          scripts/profile-rust-tests\n\n          cargo test --locked --all-targets\n",
    )?;

    assert!(!fixture.run(&[])?.status.success());
    Ok(())
}

#[cfg(unix)]
#[test]
fn gate_rejects_a_full_workload_after_a_more_indented_folded_scalar_line(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    std::fs::write(
        &fixture.workflow,
        "jobs:\n  rust-test:\n    timeout-minutes: 4\n    steps:\n      - run: scripts/profile-rust-tests\n      - run: >\n          echo setup\n            cargo test --locked --all-targets\n",
    )?;

    assert!(!fixture.run(&[])?.status.success());
    Ok(())
}

#[cfg(unix)]
#[test]
fn gate_rejects_the_command_wrapper_for_the_full_workload()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    std::fs::write(
        &fixture.workflow,
        "jobs:\n  rust-test:\n    timeout-minutes: 4\n    steps:\n      - run: scripts/profile-rust-tests\n      - run: command cargo test --locked --all-targets\n",
    )?;

    assert!(!fixture.run(&[])?.status.success());
    Ok(())
}

#[cfg(unix)]
#[test]
fn gate_rejects_an_environment_prefixed_full_workload(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    std::fs::write(
        &fixture.workflow,
        "jobs:\n  rust-test:\n    timeout-minutes: 4\n    steps:\n      - run: scripts/profile-rust-tests\n      - run: FLAG=1 cargo test --locked --all-targets\n",
    )?;

    assert!(!fixture.run(&[])?.status.success());
    Ok(())
}

#[cfg(unix)]
#[test]
fn gate_rejects_an_env_wrapper_for_the_full_workload() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    std::fs::write(
        &fixture.workflow,
        "jobs:\n  rust-test:\n    timeout-minutes: 4\n    steps:\n      - run: scripts/profile-rust-tests\n      - run: env FLAG=1 cargo test --locked --all-targets\n",
    )?;

    assert!(!fixture.run(&[])?.status.success());
    Ok(())
}

#[cfg(unix)]
#[test]
fn gate_rejects_env_option_wrapped_full_workloads() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    for command in [
        "env -C . cargo test --locked --all-targets",
        "env -S 'cargo test --locked --all-targets'",
        "env --split-string='cargo test --locked --all-targets'",
        "env -S cargo test --locked --all-targets",
        "env -S 'cargo test' --locked --all-targets",
        "env -a cargo0 cargo test --locked --all-targets",
        "env -S'cargo test --locked --all-targets'",
        "env -S '-C . cargo test --locked --all-targets'",
        "env -S '-- cargo test --locked --all-targets'",
    ] {
        std::fs::write(
            &fixture.workflow,
            format!(
                "jobs:\n  rust-test:\n    timeout-minutes: 4\n    steps:\n      - run: scripts/profile-rust-tests\n      - run: {command}\n"
            ),
        )?;
        assert!(!fixture.run(&[])?.status.success(), "{command}");
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn gate_rejects_an_exec_prefixed_full_workload()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    std::fs::write(
        &fixture.workflow,
        "jobs:\n  rust-test:\n    timeout-minutes: 4\n    steps:\n      - run: scripts/profile-rust-tests\n      - run: exec cargo test --locked --all-targets\n",
    )?;

    assert!(!fixture.run(&[])?.status.success());
    Ok(())
}

#[cfg(unix)]
#[test]
fn gate_rejects_the_command_end_of_options_wrapper_for_the_full_workload(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    std::fs::write(
        &fixture.workflow,
        "jobs:\n  rust-test:\n    timeout-minutes: 4\n    steps:\n      - run: scripts/profile-rust-tests\n      - run: command -- cargo test --locked --all-targets\n",
    )?;

    assert!(!fixture.run(&[])?.status.success());
    Ok(())
}

#[cfg(unix)]
#[test]
fn gate_rejects_the_command_path_wrapper_for_the_full_workload(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    std::fs::write(
        &fixture.workflow,
        "jobs:\n  rust-test:\n    timeout-minutes: 4\n    steps:\n      - run: scripts/profile-rust-tests\n      - run: command -p cargo test --locked --all-targets\n",
    )?;

    assert!(!fixture.run(&[])?.status.success());
    Ok(())
}
